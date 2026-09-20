use anyhow::{anyhow, Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use midir::{MidiOutput, MidiOutputConnection};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs},
    Terminal,
};
use rosc::{decoder::decode_udp, encoder::encode, OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    env, fs, io,
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const VERSION: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pattern {
    pub name: String,
    pub degrees: Vec<i32>,
    pub durations: Vec<f32>,
    pub velocities: Vec<u8>,
    pub channel: u8,
    pub destination: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub name: String,
    pub tempo: f32,
    pub pattern: Pattern,
}

fn example_project() -> Project {
    Project {
        name: "first-light".into(),
        tempo: 120.0,
        pattern: Pattern {
            name: "bass".into(),
            degrees: vec![0, 0, 3, 5, 3, 0, -2, -5],
            durations: vec![0.5; 8],
            velocities: vec![105, 90, 110, 100, 90, 105, 95, 85],
            channel: 1,
            destination: "Monitor".into(),
        },
    }
}
fn load_project(dir: &Path) -> Result<Project> {
    let p: Project = toml::from_str(&fs::read_to_string(dir.join("project.toml"))?)?;
    validate(&p.pattern)?;
    Ok(p)
}
fn save_project(dir: &Path, p: &Project) -> Result<()> {
    fs::create_dir_all(dir.join("patterns"))?;
    let data = toml::to_string_pretty(p)?;
    let tmp = dir.join("project.toml.tmp");
    fs::write(&tmp, data)?;
    fs::rename(tmp, dir.join("project.toml"))?;
    fs::write(
        dir.join("patterns/bass.toml"),
        toml::to_string_pretty(&p.pattern)?,
    )?;
    Ok(())
}
fn validate(x: &Pattern) -> Result<()> {
    if x.name.is_empty()
        || x.degrees.is_empty()
        || x.degrees.len() != x.durations.len()
        || x.degrees.len() != x.velocities.len()
    {
        return Err(anyhow!(
            "pattern requires equally-sized non-empty degrees, durations and velocities"
        ));
    }
    if x.durations.iter().any(|d| *d <= 0.0) {
        return Err(anyhow!("durations must be positive"));
    }
    if x.velocities.iter().any(|v| *v > 127) || !(1..=16).contains(&x.channel) {
        return Err(anyhow!("MIDI velocity/channel out of range"));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct MidiEvent {
    at: f64,
    kind: String,
    note: i32,
    velocity: u8,
    channel: u8,
    destination: String,
}
#[derive(Debug, Clone)]
struct AppState {
    project: Project,
    screen: usize,
    selected: usize,
    column: usize,
    playing: bool,
    engine_ready: bool,
    monitor: VecDeque<MidiEvent>,
    errors: VecDeque<String>,
    status: String,
    pending_edit: Option<String>,
}
impl AppState {
    fn new(project: Project) -> Self {
        Self {
            project,
            screen: 1,
            selected: 0,
            column: 0,
            playing: false,
            engine_ready: false,
            monitor: VecDeque::with_capacity(16),
            errors: VecDeque::with_capacity(8),
            status: "starting engine".into(),
            pending_edit: None,
        }
    }
    fn event(&mut self, e: MidiEvent) {
        if self.monitor.len() == 12 {
            self.monitor.pop_front();
        }
        self.monitor.push_back(e);
    }
    fn error(&mut self, s: String) {
        if self.errors.len() == 8 {
            self.errors.pop_front();
        }
        self.errors.push_back(s);
    }
}

struct Engine {
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    child: Child,
}
impl Engine {
    fn start(project_dir: &Path) -> Result<(Self, mpsc::Receiver<OscPacket>)> {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0")?);
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        let local = socket.local_addr()?;
        let sc_socket = UdpSocket::bind("127.0.0.1:0")?;
        let sc_addr = sc_socket.local_addr()?;
        drop(sc_socket);
        let sc = env::var("INDEX_SCLANG").ok().or_else(|| which("sclang"));
        let sc = sc.ok_or_else(|| {
            anyhow!("sclang not found; install SuperCollider or set INDEX_SCLANG=/path/to/sclang")
        })?;
        let script = fs::canonicalize("sc/bootstrap.scd").context("sc/bootstrap.scd missing")?;
        let mut child = Command::new(sc)
            .arg("-D")
            .arg(script)
            .env("INDEX_PROJECT", project_dir)
            .env("INDEX_RUST_PORT", local.port().to_string())
            .env("INDEX_SC_PORT", sc_addr.port().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("launch sclang")?;
        let (tx, rx) = mpsc::channel();
        let read_socket = socket.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 65535];
            loop {
                match read_socket.recv_from(&mut buf) {
                    Ok((n, _)) => {
                        if let Ok((_, packet)) = decode_udp(&buf[..n]) {
                            let _ = tx.send(packet);
                        }
                    }
                    Err(_) => thread::sleep(Duration::from_millis(20)),
                }
            }
        });
        if let Some(out) = child.stdout.take() {
            thread::spawn(move || {
                use std::io::BufRead;
                for line in io::BufReader::new(out).lines().flatten() {
                    eprintln!("[sclang] {line}");
                }
            });
        }
        if let Some(err) = child.stderr.take() {
            thread::spawn(move || {
                use std::io::BufRead;
                for line in io::BufReader::new(err).lines().flatten() {
                    eprintln!("[sclang stderr] {line}");
                }
            });
        }
        Ok((
            Self {
                socket,
                addr: sc_addr,
                child,
            },
            rx,
        ))
    }
    fn send(&self, path: &str, args: Vec<OscType>) -> Result<()> {
        let packet = OscPacket::Message(OscMessage {
            addr: format!("/index/{VERSION}{path}"),
            args,
        });
        self.socket.send_to(&encode(&packet)?, self.addr)?;
        Ok(())
    }
    fn stop(&mut self) {
        let _ = self.send("/transport/stop", vec![]);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
    }
}

fn which(name: &str) -> Option<String> {
    env::var_os("PATH")
        .and_then(|p| {
            env::split_paths(&p)
                .map(|x| x.join(name))
                .find(|p| p.is_file())
        })
        .map(|p| p.to_string_lossy().into_owned())
}
fn f(v: &OscType) -> Option<f32> {
    match v {
        OscType::Float(x) => Some(*x),
        OscType::Double(x) => Some(*x as f32),
        OscType::Int(x) => Some(*x as f32),
        _ => None,
    }
}
fn i(v: &OscType) -> Option<i32> {
    match v {
        OscType::Int(x) => Some(*x),
        OscType::Float(x) => Some(*x as i32),
        _ => None,
    }
}
fn s(v: &OscType) -> Option<String> {
    if let OscType::String(x) = v {
        Some(x.clone())
    } else {
        None
    }
}

fn packet_loop(rx: mpsc::Receiver<OscPacket>, state: Arc<Mutex<AppState>>) {
    for packet in rx {
        let OscPacket::Message(m) = packet else {
            continue;
        };
        let mut st = state.lock().unwrap();
        match m.addr.as_str() {
            "/index/v1/ready" => {
                st.engine_ready = true;
                st.status = "SC READY".into();
            }
            "/index/v1/state" => {
                if let Some(x) = m.args.first().and_then(i) {
                    st.playing = x != 0;
                }
            }
            "/index/v1/event/midi" => {
                if m.args.len() >= 6 {
                    if let (Some(at), Some(kind), Some(note), Some(vel), Some(ch), Some(dest)) = (
                        m.args.get(0).and_then(f).map(|x| x as f64),
                        m.args.get(1).and_then(s),
                        m.args.get(2).and_then(i),
                        m.args.get(3).and_then(i),
                        m.args.get(4).and_then(i),
                        m.args.get(5).and_then(s),
                    ) {
                        st.event(MidiEvent {
                            at,
                            kind,
                            note,
                            velocity: vel as u8,
                            channel: ch as u8,
                            destination: dest,
                        });
                    }
                }
            }
            "/index/v1/error" => {
                if let Some(x) = m.args.first().and_then(s) {
                    st.error(x);
                }
            }
            _ => {}
        }
    }
}

fn pattern_args(p: &Pattern) -> Vec<OscType> {
    vec![
        OscType::String(p.name.clone()),
        OscType::String(
            p.degrees
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        OscType::String(
            p.durations
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(","),
        ),
        OscType::String(
            p.velocities
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        OscType::Int(p.channel as i32),
        OscType::String(p.destination.clone()),
    ]
}
fn send_pattern(engine: &Engine, p: &Pattern, quantized: bool) -> Result<()> {
    engine.send("/pattern/set", pattern_args(p))?;
    engine.send("/pattern/commit", vec![OscType::Int(quantized as i32)])
}

fn draw<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, st: &AppState) -> Result<()> {
    terminal.draw(|frame| {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(5),
                    Constraint::Length(2),
                ]
                .as_ref(),
            )
            .split(frame.area());
        let tabs = Tabs::new(
            ["F1 Perform", "F2 Sequence", "F3 Devices", "F4 Code/Log"]
                .iter()
                .map(|x| Line::from(*x))
                .collect::<Vec<_>>(),
        )
        .select(st.screen)
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(tabs, root[0]);
        if st.screen == 1 {
            let header = Row::new(["#", "DEG", "DUR", "VEL", "CH", "DEST"])
                .style(Style::default().fg(Color::Yellow));
            let rows = st.project.pattern.degrees.iter().enumerate().map(|(n, d)| {
                Row::new([
                    n.to_string(),
                    d.to_string(),
                    format!("{:.2}", st.project.pattern.durations[n]),
                    st.project.pattern.velocities[n].to_string(),
                    st.project.pattern.channel.to_string(),
                    st.project.pattern.destination.clone(),
                ])
                .style(if n == st.selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                })
            });
            frame.render_widget(
                Table::new(
                    rows,
                    [
                        Constraint::Length(3),
                        Constraint::Length(5),
                        Constraint::Length(7),
                        Constraint::Length(5),
                        Constraint::Length(4),
                        Constraint::Min(10),
                    ],
                )
                .block(
                    Block::default()
                        .title(" bass / indexed pattern ")
                        .borders(Borders::ALL),
                )
                .header(header),
                root[1],
            );
        } else {
            let text = if st.screen == 0 {
                format!(
                    "{}\n\nRecent events: {}",
                    if st.playing {
                        "Transport PLAYING"
                    } else {
                        "Transport STOPPED"
                    },
                    st.monitor
                        .iter()
                        .rev()
                        .take(6)
                        .map(|e| format!(
                            "{:.2} {} n{} v{} ch{} {}",
                            e.at, e.kind, e.note, e.velocity, e.channel, e.destination
                        ))
                        .collect::<Vec<_>>()
                        .join("  ")
                )
            } else if st.screen == 2 {
                "MIDI destinations\n\nMonitor (built-in)\nNo physical port selected".into()
            } else {
                format!(
                    "Engine log\n\n{}",
                    st.errors.iter().cloned().collect::<Vec<_>>().join("\n")
                )
            };
            frame.render_widget(
                Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
                root[1],
            );
        }
        let mode = if st.pending_edit.is_some() {
            " EDIT (type value, Enter commits)"
        } else {
            " arrows/hjkl move  Enter edit  Space play/stop  : commands  ? help  q quit"
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(
                        "ENGINE {}  MIDI: Monitor  {:.2} BPM  {}",
                        if st.engine_ready {
                            "● SC READY"
                        } else {
                            "○ SC STARTING"
                        },
                        st.project.tempo,
                        if st.playing { "PLAYING" } else { "STOPPED" }
                    ),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(mode),
            ])),
            root[2],
        );
    })?;
    Ok(())
}

fn run(project_dir: PathBuf, headless: bool) -> Result<()> {
    let project = if project_dir.join("project.toml").exists() {
        load_project(&project_dir)?
    } else {
        let p = example_project();
        save_project(&project_dir, &p)?;
        p
    };
    let state = Arc::new(Mutex::new(AppState::new(project)));
    let (mut engine, rx) = Engine::start(&project_dir)?;
    let listener_state = state.clone();
    thread::spawn(move || packet_loop(rx, listener_state));
    let deadline = headless.then(|| Instant::now() + Duration::from_secs(4));
    engine.send(
        "/hello",
        vec![OscType::String("rust".into()), OscType::Int(1)],
    )?;
    let midi: Option<MidiOutputConnection> = None;
    let _ = MidiOutput::new("Index").map(|m| {
        let _ = m.ports();
    });
    if headless {
        let ready_deadline = Instant::now() + Duration::from_secs(20);
        while !state.lock().unwrap().engine_ready && Instant::now() < ready_deadline {
            engine.send(
                "/hello",
                vec![OscType::String("rust".into()), OscType::Int(1)],
            )?;
            thread::sleep(Duration::from_millis(50));
        }
        if !state.lock().unwrap().engine_ready {
            return Err(anyhow!("sclang launched but readiness handshake timed out"));
        }
        let mut p = state.lock().unwrap().project.pattern.clone();
        p.degrees[0] = 1;
        send_pattern(&engine, &p, false)?;
        engine.send(
            "/code/eval",
            vec![OscType::String("nil.doesNotExist".into())],
        )?;
        thread::sleep(Duration::from_millis(200));
        if state.lock().unwrap().errors.is_empty() {
            return Err(anyhow!("SC evaluation error was not reported"));
        }
        engine.send("/transport/play", vec![])?;
        thread::sleep(Duration::from_millis(1800));
        let got = !state.lock().unwrap().monitor.is_empty();
        engine.send("/transport/stop", vec![])?;
        if !got {
            return Err(anyhow!("smoke run received no MIDI monitor events"));
        }
        save_project(&project_dir, &state.lock().unwrap().project)?;
        drop(midi);
        return Ok(());
    }
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;
    loop {
        draw(&mut terminal, &state.lock().unwrap())?;
        if deadline.is_some_and(|d| Instant::now() > d) {
            break;
        }
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if handle_key(k, &state, &engine)? {
                    break;
                }
            }
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    engine.stop();
    Ok(())
}
fn handle_key(k: KeyEvent, state: &Arc<Mutex<AppState>>, engine: &Engine) -> Result<bool> {
    let mut st = state.lock().unwrap();
    if st.pending_edit.is_some() {
        match k.code {
            KeyCode::Enter => {
                let edit = st.pending_edit.clone().unwrap();
                if edit.starts_with(':') {
                    match edit.trim_start_matches(':').trim() {
                        "play" => {
                            st.playing = true;
                            engine.send("/transport/play", vec![])?;
                        }
                        "stop" => {
                            st.playing = false;
                            engine.send("/transport/stop", vec![])?;
                        }
                        "commit" => send_pattern(engine, &st.project.pattern, false)?,
                        _ => st.status = "commands: :play  :stop  :commit".into(),
                    }
                    st.pending_edit = None;
                    return Ok(false);
                }
                let idx = st.selected;
                match st.column {
                    0 => {
                        st.project.pattern.degrees[idx] =
                            edit.parse().context("degree must be an integer")?
                    }
                    1 => {
                        st.project.pattern.durations[idx] =
                            edit.parse().context("duration must be a number")?
                    }
                    2 => {
                        st.project.pattern.velocities[idx] =
                            edit.parse().context("velocity must be 0..127")?
                    }
                    3 => {
                        st.project.pattern.channel =
                            edit.parse().context("channel must be 1..16")?
                    }
                    4 => st.project.pattern.destination = edit,
                    _ => {}
                }
                validate(&st.project.pattern)?;
                st.pending_edit = None;
                send_pattern(engine, &st.project.pattern, false)?;
            }
            KeyCode::Esc => st.pending_edit = None,
            KeyCode::Backspace => {
                st.pending_edit.as_mut().unwrap().pop();
            }
            KeyCode::Char(c) => st.pending_edit.as_mut().unwrap().push(c),
            _ => {}
        }
        return Ok(false);
    }
    match k.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('?') => {
            st.status = "Enter edits DEG only in this slice; arrows select rows".into()
        }
        KeyCode::Up | KeyCode::Char('k') => st.selected = st.selected.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            st.selected = (st.selected + 1).min(st.project.pattern.degrees.len() - 1)
        }
        KeyCode::Left | KeyCode::Char('h') => st.column = st.column.saturating_sub(1),
        KeyCode::Right | KeyCode::Char('l') => st.column = (st.column + 1).min(4),
        KeyCode::F(1) => st.screen = 0,
        KeyCode::F(2) => st.screen = 1,
        KeyCode::F(3) => st.screen = 2,
        KeyCode::F(4) => st.screen = 3,
        KeyCode::Tab => st.screen = (st.screen + 1) % 4,
        KeyCode::Enter if st.screen == 1 => {
            st.pending_edit = Some(match st.column {
                0 => st.project.pattern.degrees[st.selected].to_string(),
                1 => st.project.pattern.durations[st.selected].to_string(),
                2 => st.project.pattern.velocities[st.selected].to_string(),
                3 => st.project.pattern.channel.to_string(),
                _ => st.project.pattern.destination.clone(),
            })
        }
        KeyCode::Char(':') => st.pending_edit = Some(":".into()),
        KeyCode::Char(' ') => {
            st.playing = !st.playing;
            engine.send(
                if st.playing {
                    "/transport/play"
                } else {
                    "/transport/stop"
                },
                vec![],
            )?;
        }
        _ => {}
    }
    Ok(false)
}

fn doctor() {
    println!("Index doctor");
    match which("sclang").or_else(|| env::var("INDEX_SCLANG").ok()) {
        Some(p) => println!("sclang: {p}"),
        None => {
            println!("sclang: MISSING");
            println!("Install SuperCollider with: brew install supercollider");
            println!("Or configure: export INDEX_SCLANG=/absolute/path/to/sclang");
        }
    }
    println!("MIDI: native midir backend available (Monitor is always available)");
}
fn main() -> Result<()> {
    let mut a = env::args().skip(1);
    match a.next().as_deref() {
        Some("doctor") => {
            doctor();
            Ok(())
        }
        Some("run") => {
            let dir = PathBuf::from(a.next().unwrap_or_else(|| "examples/first-light".into()));
            run(dir, a.any(|x| x == "--headless"))
        }
        Some("test-project") => {
            let dir = PathBuf::from(a.next().unwrap_or_else(|| ".index-test".into()));
            let p = example_project();
            save_project(&dir, &p)?;
            println!("saved and reloaded: {}", load_project(&dir)?.name);
            Ok(())
        }
        _ => {
            println!("Index — sequencing-only Rust/Ratatui + SuperCollider instrument\n\nUsage:\n  cargo run -- doctor\n  cargo run -- run examples/first-light\n  cargo run -- run examples/first-light --headless");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pattern_validates() {
        let p = example_project().pattern;
        assert!(validate(&p).is_ok());
        let mut q = p.clone();
        q.durations.pop();
        assert!(validate(&q).is_err());
    }
    #[test]
    fn project_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = example_project();
        save_project(d.path(), &p).unwrap();
        assert_eq!(load_project(d.path()).unwrap(), p);
    }
    #[test]
    fn osc_roundtrip() {
        let p = OscPacket::Message(OscMessage {
            addr: "/index/v1/hello".into(),
            args: vec![OscType::String("x".into()), OscType::Int(1)],
        });
        let b = encode(&p).unwrap();
        let (_, x) = decode_udp(&b).unwrap();
        assert_eq!(x, p);
    }
    #[test]
    fn reducer_moves_selection() {
        let st = Arc::new(Mutex::new(AppState::new(example_project())));
        let sock = Arc::new(UdpSocket::bind("127.0.0.1:0").unwrap());
        let e = Engine {
            socket: sock.clone(),
            addr: sock.local_addr().unwrap(),
            child: Command::new("true").spawn().unwrap(),
        };
        handle_key(
            KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE),
            &st,
            &e,
        )
        .unwrap();
        assert_eq!(st.lock().unwrap().selected, 1);
    }
}
