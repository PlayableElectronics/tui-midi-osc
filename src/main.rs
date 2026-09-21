mod midi;
mod project;
mod protocol;

use anyhow::{anyhow, Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use midi::{MidiOutputBackend, MonitorOutput, OutputQueue, SharedOutput};
use project::{example_project, load, save, validate, Pattern, Project};
use protocol::*;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
    Terminal,
};
use rosc::{decoder::decode_udp, encoder::encode, OscMessage, OscPacket, OscType};
use std::{
    collections::VecDeque,
    env, fs, io,
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineStatus {
    Starting,
    Syncing,
    Ready,
    Error,
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
    status: EngineStatus,
    screen: usize,
    selected: usize,
    column: usize,
    device_index: usize,
    playing: bool,
    staged: bool,
    live_revision: u64,
    staged_revision: u64,
    pending_edit: Option<String>,
    monitor: VecDeque<MidiEvent>,
    logs: VecDeque<String>,
    errors: VecDeque<String>,
    request: u64,
    local_epoch: Instant,
    sc_offset: f64,
}
impl AppState {
    fn new(project: Project) -> Self {
        Self {
            project,
            status: EngineStatus::Starting,
            screen: 1,
            selected: 0,
            column: 0,
            device_index: 0,
            playing: false,
            staged: false,
            live_revision: 0,
            staged_revision: 0,
            pending_edit: None,
            monitor: VecDeque::new(),
            logs: VecDeque::new(),
            errors: VecDeque::new(),
            request: 1,
            local_epoch: Instant::now(),
            sc_offset: 0.0,
        }
    }
    fn next_request(&mut self) -> String {
        let x = format!("r{}", self.request);
        self.request += 1;
        x
    }
    fn error(&mut self, text: impl Into<String>) {
        if self.errors.len() >= 32 {
            self.errors.pop_front();
        }
        self.errors.push_back(text.into());
    }
    fn log(&mut self, text: impl Into<String>) {
        if self.logs.len() >= 64 {
            self.logs.pop_front();
        }
        self.logs.push_back(text.into());
    }
    fn event(&mut self, e: MidiEvent) {
        if self.monitor.len() >= 24 {
            self.monitor.pop_front();
        }
        self.monitor.push_back(e);
    }
}

struct Engine {
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    child: Child,
    stopped: bool,
}
impl Engine {
    fn start() -> Result<(Self, mpsc::Receiver<OscPacket>, mpsc::Receiver<String>)> {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0")?);
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        let rust_addr = socket.local_addr()?;
        let reserve = UdpSocket::bind("127.0.0.1:0")?;
        let sc_addr = reserve.local_addr()?;
        drop(reserve);
        let path = env::var("INDEX_SCLANG").ok().or_else(|| which("sclang")).ok_or_else(|| anyhow!("sclang not found; install with `brew install --cask supercollider` or set INDEX_SCLANG=/absolute/path/to/sclang"))?;
        let script = fs::canonicalize("sc/bootstrap.scd").context("sc/bootstrap.scd missing")?;
        let mut child = Command::new(path)
            .arg("-D")
            .arg(script)
            .env("INDEX_RUST_PORT", rust_addr.port().to_string())
            .env("INDEX_SC_PORT", sc_addr.port().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("launch sclang")?;
        let (tx, rx) = mpsc::channel();
        let recv = socket.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 65535];
            loop {
                match recv.recv_from(&mut buf) {
                    Ok((n, _)) => {
                        if let Ok((_, packet)) = decode_udp(&buf[..n]) {
                            let _ = tx.send(packet);
                        }
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        });
        let (ltx, lrx) = mpsc::channel();
        if let Some(pipe) = child.stdout.take() {
            let tx = ltx.clone();
            thread::spawn(move || {
                use io::BufRead;
                for line in io::BufReader::new(pipe).lines().map_while(Result::ok) {
                    let _ = tx.send(format!("SC stdout: {line}"));
                }
            });
        }
        if let Some(pipe) = child.stderr.take() {
            let tx = ltx.clone();
            thread::spawn(move || {
                use io::BufRead;
                for line in io::BufReader::new(pipe).lines().map_while(Result::ok) {
                    let _ = tx.send(format!("SC stderr: {line}"));
                }
            });
        }
        Ok((
            Self {
                socket,
                addr: sc_addr,
                child,
                stopped: false,
            },
            rx,
            lrx,
        ))
    }
    fn send(&self, path: &str, args: Vec<OscType>) -> Result<()> {
        let p = OscPacket::Message(OscMessage {
            addr: format!("{ROOT}{path}"),
            args,
        });
        self.socket.send_to(&encode(&p)?, self.addr)?;
        Ok(())
    }
    fn exited(&mut self) -> Result<Option<i32>> {
        Ok(self.child.try_wait()?.map(|s| s.code().unwrap_or(-1)))
    }
    fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
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

fn pattern_args(req: &str, rev: u64, p: &Pattern) -> Vec<OscType> {
    protocol::pattern_args(req, rev, p)
}
fn send_sync(e: &Engine, s: &mut AppState) -> Result<()> {
    let req = s.next_request();
    s.status = EngineStatus::Syncing;
    let mut a = vec![OscType::String(req), OscType::Float(s.project.meta.tempo)];
    a.extend(
        pattern_args("sync", 1, &s.project.pattern)
            .into_iter()
            .skip(2),
    );
    e.send("/project/sync", a)
}
fn stage(e: &Engine, s: &mut AppState) -> Result<()> {
    if s.status != EngineStatus::Ready {
        return Err(anyhow!("SC is not synchronized yet"));
    }
    let req = s.next_request();
    s.staged = true;
    s.staged_revision += 1;
    e.send(
        "/pattern/set",
        pattern_args(&req, s.staged_revision, &s.project.pattern),
    )
}
fn commit(e: &Engine, s: &mut AppState, mode: &str) -> Result<()> {
    if s.status != EngineStatus::Ready {
        return Err(anyhow!("SC is not synchronized yet"));
    }
    if mode != "now" && mode != "bar" {
        return Err(anyhow!("commit mode must be now or bar"));
    }
    let req = s.next_request();
    e.send(
        "/pattern/commit",
        vec![
            OscType::String(req),
            OscType::Int(s.staged_revision as i32),
            OscType::String(mode.into()),
        ],
    )
}

fn handle_packet(p: OscPacket, s: &mut AppState, queue: &OutputQueue) {
    let OscPacket::Message(m) = p else { return };
    match m.addr.as_str() {
        READY => {
            if s.status == EngineStatus::Starting {
                if m.args.first() != Some(&OscType::Int(1)) {
                    s.error("protocol error: unsupported SC protocol version");
                    s.status = EngineStatus::Error;
                    return;
                }
                if let Some(OscType::Float(sc_time)) = m.args.get(1) {
                    s.sc_offset = s.local_epoch.elapsed().as_secs_f64() - (*sc_time as f64);
                }
                s.status = EngineStatus::Syncing;
                s.log("SC READY; synchronizing project");
            }
        }
        SYNC_ACK => {
            if s.status != EngineStatus::Syncing {
                return;
            }
            s.status = EngineStatus::Ready;
            s.staged = false;
            s.staged_revision = 1;
            s.live_revision = m
                .args
                .get(1)
                .and_then(|x| {
                    if let OscType::Int(v) = x {
                        Some(*v as u64)
                    } else {
                        None
                    }
                })
                .unwrap_or(1);
            s.log("startup project synchronized and activated")
        }
        STAGED => {
            s.staged = true;
            s.staged_revision = m
                .args
                .get(1)
                .and_then(|x| {
                    if let OscType::Int(v) = x {
                        Some(*v as u64)
                    } else {
                        None
                    }
                })
                .unwrap_or(s.staged_revision);
            s.log("pattern staged")
        }
        COMMITTED => {
            s.staged = false;
            s.live_revision = m
                .args
                .get(1)
                .and_then(|x| {
                    if let OscType::Int(v) = x {
                        Some(*v as u64)
                    } else {
                        None
                    }
                })
                .unwrap_or(s.live_revision);
            s.log("pattern activated")
        }
        "/index/v1/state" => {
            if let Some(OscType::Int(v)) = m.args.first() {
                s.playing = *v != 0
            }
        }
        EVENT => {
            if m.args.len() != 6 {
                s.error("malformed MIDI event");
                return;
            }
            let parse = (
                one_float(&m.args, 0),
                one_string(&m.args, 1),
                one_int(&m.args, 2),
                one_int(&m.args, 3),
                one_int(&m.args, 4),
                one_string(&m.args, 5),
            );
            if let (Ok(at), Ok(kind), Ok(note), Ok(vel), Ok(ch), Ok(dest)) = parse {
                let ev = MidiEvent {
                    at: at as f64,
                    kind,
                    note,
                    velocity: vel as u8,
                    channel: ch as u8,
                    destination: dest,
                };
                let due = (ev.at + s.sc_offset - s.local_epoch.elapsed().as_secs_f64()).max(0.0);
                if let Err(x) = queue.send_at(
                    Instant::now() + Duration::from_secs_f64(due),
                    ev.kind.clone(),
                    ev.note,
                    ev.velocity,
                    ev.channel,
                ) {
                    s.error(format!("MIDI output: {x}"));
                }
                s.event(ev)
            } else {
                s.error("malformed MIDI event types")
            }
        }
        ERROR => {
            let category = m
                .args
                .get(1)
                .and_then(|x| {
                    if let OscType::String(v) = x {
                        Some(v.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("protocol");
            let message = m
                .args
                .get(2)
                .and_then(|x| {
                    if let OscType::String(v) = x {
                        Some(v.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "SC protocol error".into());
            s.error(format!("SC {category}: {message}"));
        }
        _ => s.error(format!("unknown SC message {}", m.addr)),
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    active: bool,
}
impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen)?;
        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(out))?,
            active: true,
        })
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
            let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
            let _ = self.terminal.show_cursor();
            self.active = false;
        }
    }
}

fn draw(g: &mut Terminal<CrosstermBackend<io::Stdout>>, s: &AppState) -> Result<()> {
    g.draw(|f| {
        let z = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(5), Constraint::Length(2)]).split(f.area());
        f.render_widget(Tabs::new(["F1 Perform", "F2 Sequence", "F3 Devices", "F4 Log"].iter().map(|x| Line::from(*x)).collect::<Vec<_>>()).select(s.screen).block(Block::default().borders(Borders::BOTTOM)), z[0]);
        if s.screen == 1 {
            let rows = s.project.pattern.degrees.iter().enumerate().map(|(n, d)| { let v = [n.to_string(), d.to_string(), format!("{:.2}", s.project.pattern.durations[n]), s.project.pattern.velocities[n].to_string(), s.project.pattern.channel.to_string(), s.project.pattern.destination.clone()]; Row::new(v.into_iter().enumerate().map(|(c, x)| Cell::from(x).style(if n == s.selected && c == s.column + 1 { Style::default().bg(Color::Yellow).fg(Color::Black) } else if n == s.selected { Style::default().bg(Color::DarkGray) } else { Style::default() }))) });
            f.render_widget(Table::new(rows, [Constraint::Length(3), Constraint::Length(5), Constraint::Length(7), Constraint::Length(5), Constraint::Length(4), Constraint::Min(10)]).header(Row::new(["#", "DEG", "DUR", "VEL", "CH", "DEST"]).style(Style::default().fg(Color::Yellow))).block(Block::default().title(format!(" bass / {} ", if s.staged { "STAGED" } else { "LIVE" })).borders(Borders::ALL)), z[1]);
        } else {
            let text = if s.screen == 0 { format!("Transport {}\n\n{}", if s.playing { "PLAYING" } else { "STOPPED" }, s.monitor.iter().rev().take(8).map(|e| format!("{:.2} {} n{} v{} ch{} {}", e.at, e.kind, e.note, e.velocity, e.channel, e.destination)).collect::<Vec<_>>().join("  ")) } else if s.screen == 2 { let mut names = vec!["Monitor (built-in)".to_string()]; names.extend(MidiOutputBackend::ports().map(|p| p.into_iter().map(|x| x.0).collect::<Vec<_>>()).unwrap_or_default()); format!("MIDI destinations (j/k, Enter selects)\n\n{}", names.into_iter().enumerate().map(|(i, n)| format!("{} {}", if i == s.device_index { ">" } else { " " }, n)).collect::<Vec<_>>().join("\n")) } else { format!("Engine log\n\n{}\n{}", s.logs.iter().cloned().collect::<Vec<_>>().join("\n"), s.errors.iter().cloned().collect::<Vec<_>>().join("\n")) };
            f.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL)), z[1]);
        }
        let status = match s.status { EngineStatus::Starting => "SC STARTING", EngineStatus::Syncing => "SC SYNCING", EngineStatus::Ready => "SC READY", EngineStatus::Error => "SC ERROR" };
        f.render_widget(Paragraph::new(Line::from(vec![Span::styled(format!("ENGINE ● {}  MIDI: Monitor  {:.2} BPM  {}", status, s.project.meta.tempo, if s.playing { "PLAYING" } else { "STOPPED" }), Style::default().fg(if s.status == EngineStatus::Error { Color::Red } else { Color::Green })), Span::raw(if s.pending_edit.is_some() { " EDIT Enter=apply Esc=cancel" } else { " h/l cell  j/k row  Enter edit  i now  b next-bar  Space play  : command  ? help  q quit" })])), z[2]);
    })?;
    Ok(())
}

fn edit_key(k: KeyEvent, s: &mut AppState, e: &Engine, out: &SharedOutput) -> Result<bool> {
    if let Some(text) = s.pending_edit.clone() {
        match k.code {
            KeyCode::Enter => {
                if let Some(cmd) = text.strip_prefix(':') {
                    match cmd.trim() {
                        "play" if s.status == EngineStatus::Ready => {
                            e.send("/transport/play", vec![])?
                        }
                        "stop" => e.send("/transport/stop", vec![])?,
                        "commit" => commit(e, s, "now")?,
                        "bar" => commit(e, s, "bar")?,
                        _ => s.error("commands: :play :stop :commit :bar"),
                    };
                    s.pending_edit = None;
                    return Ok(false);
                }
                let i = s.selected;
                let mut candidate = s.project.pattern.clone();
                let r: Result<()> = match s.column {
                    0 => text
                        .parse::<i32>()
                        .map(|v| candidate.degrees[i] = v)
                        .map_err(|_| anyhow!("degree must be an integer")),
                    1 => text
                        .parse::<f32>()
                        .map(|v| candidate.durations[i] = v)
                        .map_err(|_| anyhow!("duration must be a positive number")),
                    2 => text
                        .parse::<u8>()
                        .map(|v| candidate.velocities[i] = v)
                        .map_err(|_| anyhow!("velocity must be 0..127")),
                    3 => text
                        .parse::<u8>()
                        .map(|v| candidate.channel = v)
                        .map_err(|_| anyhow!("channel must be 1..16")),
                    4 => {
                        candidate.destination = text;
                        Ok(())
                    }
                    _ => Ok(()),
                };
                if let Err(x) = r.and_then(|_| validate(&candidate)) {
                    s.error(x.to_string());
                    s.pending_edit = None;
                    return Ok(false);
                }
                s.project.pattern = candidate;
                stage(e, s)?;
                s.pending_edit = None
            }
            KeyCode::Esc => s.pending_edit = None,
            KeyCode::Backspace => {
                s.pending_edit.as_mut().unwrap().pop();
            }
            KeyCode::Char(c) => s.pending_edit.as_mut().unwrap().push(c),
            _ => {}
        }
        return Ok(false);
    }
    match k.code {
        KeyCode::Char('q') => Ok(true),
        KeyCode::Char('?') => {
            s.log("h/l select cell; Enter edit; i immediate; b next bar; :play/:stop/:commit/:bar");
            Ok(false)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if s.screen == 2 {
                s.device_index = s.device_index.saturating_sub(1);
                return Ok(false);
            }
            s.selected = s.selected.saturating_sub(1);
            Ok(false)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if s.screen == 2 {
                let max = MidiOutputBackend::ports().map(|x| x.len()).unwrap_or(0);
                s.device_index = (s.device_index + 1).min(max);
                return Ok(false);
            }
            s.selected = (s.selected + 1).min(s.project.pattern.degrees.len() - 1);
            Ok(false)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            s.column = s.column.saturating_sub(1);
            Ok(false)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            s.column = (s.column + 1).min(4);
            Ok(false)
        }
        KeyCode::F(n) => {
            s.screen = (n as usize).saturating_sub(1).min(3);
            Ok(false)
        }
        KeyCode::Tab => {
            s.screen = (s.screen + 1) % 4;
            Ok(false)
        }
        KeyCode::Enter if s.screen == 1 && s.status == EngineStatus::Ready => {
            s.pending_edit = Some(match s.column {
                0 => s.project.pattern.degrees[s.selected].to_string(),
                1 => s.project.pattern.durations[s.selected].to_string(),
                2 => s.project.pattern.velocities[s.selected].to_string(),
                3 => s.project.pattern.channel.to_string(),
                _ => s.project.pattern.destination.clone(),
            });
            Ok(false)
        }
        KeyCode::Enter if s.screen == 2 => {
            if s.device_index == 0 {
                *out.lock().unwrap() = Box::new(MonitorOutput::default());
                s.log("Monitor output selected");
            } else if let Ok(ports) = MidiOutputBackend::ports() {
                if let Some((name, port)) = ports.into_iter().nth(s.device_index - 1) {
                    match MidiOutputBackend::open(&port, name.clone()) {
                        Ok(m) => {
                            *out.lock().unwrap() = Box::new(m);
                            s.log(format!("MIDI output selected: {name}"));
                        }
                        Err(e) => s.error(format!("MIDI open failed: {e}")),
                    }
                }
            }
            Ok(false)
        }
        KeyCode::Char(':') => {
            s.pending_edit = Some(":".into());
            Ok(false)
        }
        KeyCode::Char('i') => {
            commit(e, s, "now")?;
            Ok(false)
        }
        KeyCode::Char('b') => {
            commit(e, s, "bar")?;
            Ok(false)
        }
        KeyCode::Char(' ') => {
            if s.status != EngineStatus::Ready {
                s.error("SC is not synchronized yet");
                return Ok(false);
            }
            e.send(
                if s.playing {
                    "/transport/stop"
                } else {
                    "/transport/play"
                },
                vec![],
            )?;
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn run(dir: PathBuf, headless: bool) -> Result<()> {
    let p = if dir.join("project.toml").exists() {
        load(&dir)?
    } else {
        let p = example_project(&dir);
        save(&p)?;
        p
    };
    let state = Arc::new(Mutex::new(AppState::new(p)));
    let (mut e, rx, logs) = Engine::start()?;
    let out: SharedOutput = Arc::new(Mutex::new(Box::new(MonitorOutput::default())));
    let a = state.clone();
    let queue = Arc::new(OutputQueue::new(out.clone()));
    let q = queue.clone();
    thread::spawn(move || {
        for p in rx {
            handle_packet(p, &mut a.lock().unwrap(), &q)
        }
    });
    let a = state.clone();
    thread::spawn(move || {
        for l in logs {
            a.lock().unwrap().log(l)
        }
    });
    let mut terminal = if headless {
        None
    } else {
        Some(TerminalGuard::new()?)
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last_hello = Instant::now() - Duration::from_secs(1);
    let mut last_sync = Instant::now() - Duration::from_secs(1);
    loop {
        if let Some(code) = e.exited()? {
            let mut s = state.lock().unwrap();
            s.status = EngineStatus::Error;
            s.error(format!(
                "sclang exited ({code}); recent SC output is on the Log screen"
            ));
            if headless {
                return Err(anyhow!("sclang exited before sync"));
            }
        }
        {
            let mut s = state.lock().unwrap();
            if s.status == EngineStatus::Starting
                && last_hello.elapsed() >= Duration::from_millis(250)
            {
                e.send(
                    "/hello",
                    vec![OscType::String("rust".into()), OscType::Int(1)],
                )?;
                last_hello = Instant::now();
            }
            if s.status == EngineStatus::Syncing
                && last_sync.elapsed() >= Duration::from_millis(500)
            {
                send_sync(&e, &mut s)?;
                last_sync = Instant::now();
            }
        }
        if state.lock().unwrap().status == EngineStatus::Ready && headless {
            break;
        }
        if Instant::now() > deadline {
            let mut s = state.lock().unwrap();
            s.status = EngineStatus::Error;
            let logs = format!("{:?}", s.logs);
            s.error(format!("startup timeout; logs: {logs}"));
            if headless {
                eprintln!("startup logs: {logs}; errors: {:?}", s.errors);
                return Err(anyhow!("startup synchronization timed out"));
            }
        }
        if let Some(g) = terminal.as_mut() {
            let s = state.lock().unwrap();
            draw(&mut g.terminal, &s)?;
            if event::poll(Duration::from_millis(50))?
                && matches!(
                    event::read()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    })
                )
            {
                e.stop();
                return Ok(());
            }
        } else {
            thread::sleep(Duration::from_millis(50));
        }
        if state.lock().unwrap().status == EngineStatus::Error && headless {
            return Err(anyhow!("sclang startup failed"));
        }
        if !headless && state.lock().unwrap().status == EngineStatus::Ready {
            break;
        }
    }
    if headless {
        let mut s = state.lock().unwrap();
        s.project.pattern.degrees = vec![7];
        s.project.pattern.durations = vec![0.25];
        s.project.pattern.velocities = vec![77];
        s.project.pattern.channel = 2;
        s.project.pattern.destination = "Smoke".into();
        stage(&e, &mut s)?;
        e.send("/transport/play", vec![])?;
        drop(s);
        thread::sleep(Duration::from_millis(1200));
        let before_commit = state.lock().unwrap().monitor.clone();
        if before_commit.iter().any(|x| x.destination == "Smoke") {
            return Err(anyhow!(
                "staged pattern leaked into live playback before commit"
            ));
        }
        commit(&e, &mut state.lock().unwrap(), "bar")?;
        let commit_deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < commit_deadline
            && !state
                .lock()
                .unwrap()
                .monitor
                .iter()
                .any(|x| x.destination == "Smoke")
        {
            thread::sleep(Duration::from_millis(100));
        }
        e.send(
            "/code/eval",
            vec![OscType::String("nil.doesNotExist".into())],
        )?;
        thread::sleep(Duration::from_millis(200));
        e.send("/transport/stop", vec![])?;
        thread::sleep(Duration::from_millis(300));
        let after_stop = state.lock().unwrap().monitor.clone();
        if !before_commit
            .iter()
            .any(|x| x.kind == "on" && x.note == 60 && x.channel == 1 && x.destination == "Monitor")
        {
            return Err(anyhow!("startup pattern was not observed"));
        }
        if !after_stop.iter().any(|x| {
            x.kind == "on"
                && x.note == 67
                && x.velocity == 77
                && x.channel == 2
                && x.destination == "Smoke"
        }) {
            return Err(anyhow!("committed pattern did not switch atomically"));
        }
        if state.lock().unwrap().errors.is_empty() {
            return Err(anyhow!("SC evaluation error was not reported"));
        }
        let count = after_stop.iter().filter(|x| x.kind == "on").count();
        thread::sleep(Duration::from_millis(800));
        let later_count = state
            .lock()
            .unwrap()
            .monitor
            .iter()
            .filter(|x| x.kind == "on")
            .count();
        if later_count > count {
            return Err(anyhow!("events continued after Stop"));
        }
        save(&state.lock().unwrap().project)?;
        queue.shutdown();
        return Ok(());
    }
    let mut g = terminal.take().expect("interactive terminal");
    loop {
        if let Some(code) = e.exited()? {
            queue.cancel();
            let mut s = state.lock().unwrap();
            s.status = EngineStatus::Error;
            s.playing = false;
            s.error(format!("sclang exited unexpectedly ({code})"));
        }
        {
            let mut s = state.lock().unwrap();
            for error in queue.drain_errors() {
                s.error(format!("MIDI output: {error}"));
            }
            draw(&mut g.terminal, &s)?
        }
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if edit_key(k, &mut state.lock().unwrap(), &e, &out)? {
                    break;
                }
            }
        }
    }
    e.stop();
    queue.shutdown();
    Ok(())
}

fn doctor() {
    println!("Index doctor");
    match env::var("INDEX_SCLANG").ok().or_else(|| which("sclang")) {
        Some(x) => println!("sclang: {x}"),
        None => {
            println!("sclang: MISSING");
            println!("Install: brew install --cask supercollider");
            println!("Override: export INDEX_SCLANG=/absolute/path/to/sclang")
        }
    }
    println!("MIDI: native midir backend available; Monitor is always available")
}
fn main() -> Result<()> {
    let mut a = env::args().skip(1);
    match a.next().as_deref() {
        Some("doctor") => {
            doctor();
            Ok(())
        }
        Some("run") => run(
            PathBuf::from(a.next().unwrap_or_else(|| "examples/first-light".into())),
            a.any(|x| x == "--headless"),
        ),
        _ => {
            println!("Index\n\n  cargo run -- doctor\n  cargo run -- run examples/first-light\n  ./scripts/smoke-test");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_transitions() {
        let mut s = AppState::new(example_project("."));
        assert_eq!(s.status, EngineStatus::Starting);
        s.status = EngineStatus::Syncing;
        assert_eq!(s.status, EngineStatus::Syncing);
        s.status = EngineStatus::Ready;
        assert_eq!(s.status, EngineStatus::Ready)
    }
    #[test]
    fn staged_state_is_distinct() {
        let p = example_project(".");
        let mut s = AppState::new(p.clone());
        s.project.pattern.degrees[0] = 12;
        s.staged = true;
        assert_eq!(s.project.pattern.degrees[0], 12);
        assert_eq!(p.pattern.degrees[0], 0)
    }
    #[test]
    fn engine_stop_is_idempotent() {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").unwrap());
        let child = Command::new("true").spawn().unwrap();
        let mut e = Engine {
            socket: socket.clone(),
            addr: socket.local_addr().unwrap(),
            child,
            stopped: false,
        };
        e.stop();
        e.stop();
        assert!(e.stopped);
    }
}
