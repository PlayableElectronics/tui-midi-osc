mod project;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use project::{example_project, load, note_name, save, DURATIONS, STEP_COUNT};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{
    collections::VecDeque,
    env, io,
    path::PathBuf,
    time::{Duration, Instant},
};

struct App {
    project: project::Project,
    edit_cursor: usize,
    play_cursor: Option<usize>,
    playing: bool,
    next_deadline: Instant,
    last_action: String,
    activity: VecDeque<String>,
}
impl App {
    fn new(project: project::Project) -> Self {
        Self {
            project,
            edit_cursor: 0,
            play_cursor: None,
            playing: false,
            next_deadline: Instant::now(),
            last_action: "Loaded first-light".into(),
            activity: VecDeque::new(),
        }
    }
    fn set_action(&mut self, text: impl Into<String>) {
        self.last_action = text.into();
    }
    fn move_cursor(&mut self, delta: isize) {
        self.edit_cursor =
            (self.edit_cursor as isize + delta).rem_euclid(STEP_COUNT as isize) as usize;
        self.set_action(format!("Edit step {}", self.edit_cursor + 1));
    }
    fn transpose(&mut self, amount: i16) {
        let index = self.edit_cursor;
        let note = self.project.steps[index].note.unwrap_or(60) as i16;
        self.project.steps[index].note = Some((note + amount).clamp(0, 127) as u8);
        self.set_action(format!(
            "{} -> {}",
            self.edit_cursor + 1,
            note_name(self.project.steps[index].note)
        ));
    }
    fn toggle_rest(&mut self) {
        let index = self.edit_cursor;
        self.project.steps[index].note = if self.project.steps[index].note.is_some() {
            None
        } else {
            Some(60)
        };
        self.set_action(format!(
            "{} {}",
            self.edit_cursor + 1,
            note_name(self.project.steps[index].note)
        ));
    }
    fn adjust_velocity(&mut self, amount: i16) {
        let index = self.edit_cursor;
        self.project.steps[index].velocity =
            (self.project.steps[index].velocity as i16 + amount).clamp(0, 127) as u8;
        self.set_action(format!("Velocity {}", self.project.steps[index].velocity));
    }
    fn adjust_duration(&mut self, amount: isize) {
        let index = self.edit_cursor;
        let duration = self.project.steps[index].duration;
        let current = DURATIONS
            .iter()
            .position(|x| (*x - duration).abs() < f32::EPSILON)
            .unwrap_or(1);
        let index = (current as isize + amount).clamp(0, DURATIONS.len() as isize - 1) as usize;
        self.project.steps[self.edit_cursor].duration = DURATIONS[index];
        self.set_action(format!(
            "Duration {:.3} beat",
            self.project.steps[self.edit_cursor].duration
        ));
    }
    fn toggle_play(&mut self) {
        self.playing = !self.playing;
        if self.playing {
            self.play_cursor = Some(0);
            self.next_deadline = Instant::now();
            self.set_action("PLAY Monitor");
        } else {
            self.play_cursor = None;
            self.set_action("STOP; playhead reset");
        }
    }
    fn adjust_tempo(&mut self, amount: f32) {
        self.project.tempo = (self.project.tempo + amount).clamp(30.0, 300.0);
        self.set_action(format!("Tempo {:.0} BPM", self.project.tempo));
    }
    fn tick(&mut self, now: Instant) {
        if !self.playing {
            return;
        }
        while now >= self.next_deadline {
            let index = self.play_cursor.unwrap_or(0);
            let step = &self.project.steps[index];
            if let Some(note) = step.note {
                push_activity(
                    &mut self.activity,
                    format!(
                        "MONITOR  {:02}  ON  {}  v{}  {:.3} beat",
                        index + 1,
                        note_name(Some(note)),
                        step.velocity,
                        step.duration
                    ),
                );
            }
            let seconds = 60.0 / self.project.tempo * step.duration;
            self.next_deadline += Duration::from_secs_f32(seconds.max(0.001));
            self.play_cursor = Some((index + 1) % STEP_COUNT);
        }
    }
    fn key(&mut self, key: KeyEvent) -> Result<bool> {
        let shifted = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1),
            KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1),
            KeyCode::Up | KeyCode::Char('j') if shifted || key.code == KeyCode::Char('J') => self.transpose(12),
            KeyCode::Down | KeyCode::Char('k') if shifted || key.code == KeyCode::Char('K') => self.transpose(-12),
            KeyCode::Up => self.transpose(1),
            KeyCode::Down => self.transpose(-1),
            KeyCode::Char('J') => self.transpose(12),
            KeyCode::Char('K') => self.transpose(-12),
            KeyCode::Char('[') => self.adjust_velocity(-1),
            KeyCode::Char(']') => self.adjust_velocity(1),
            KeyCode::Char('-') => self.adjust_duration(-1),
            KeyCode::Char('+') => self.adjust_duration(1),
            KeyCode::Char('r') => self.toggle_rest(),
            KeyCode::Char(' ') => self.toggle_play(),
            KeyCode::Char(',') => self.adjust_tempo(-1.0),
            KeyCode::Char('.') => self.adjust_tempo(1.0),
            KeyCode::Char('s') => match save(&self.project) { Ok(()) => self.set_action("Saved"), Err(error) => self.set_action(format!("Save error: {error}")) },
            KeyCode::Char('?') => self.set_action("h/l move  j/k transpose  J/K octave  [/] velocity  -/+ duration  r rest  Space play  ,/. tempo  s save  q quit"),
            _ => {}
        }
        Ok(false)
    }
}
fn push_activity(log: &mut VecDeque<String>, line: String) {
    if log.len() >= 5 {
        log.pop_front();
    }
    log.push_back(line);
}
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}
impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut out = io::stdout();
        if let Err(error) = execute!(out, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        let terminal = Terminal::new(CrosstermBackend::new(out)).context("initialize terminal")?;
        Ok(Self { terminal })
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn duration_mark(duration: f32) -> &'static str {
    match duration {
        x if (x - 0.125).abs() < f32::EPSILON => "·",
        x if (x - 0.25).abs() < f32::EPSILON => "▬",
        x if (x - 0.5).abs() < f32::EPSILON => "▬▬",
        _ => "▬▬▬▬",
    }
}
fn draw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &App) -> Result<()> {
    terminal.draw(|frame| {
        let areas = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(8), Constraint::Length(7), Constraint::Length(2)]).split(frame.area());
        let header = format!(" INDEX / FIRST-LIGHT     {}     {:03.0} BPM     {}     EDIT {:02}     PLAY {:02}", if app.playing { "● PLAY" } else { "○ STOP" }, app.project.tempo, app.last_action, app.edit_cursor + 1, app.play_cursor.map(|x| x + 1).unwrap_or(0));
        frame.render_widget(Paragraph::new(header).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)).block(Block::default().borders(Borders::ALL).title(" TRANSPORT ")), areas[0]);
        let grid = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Ratio(1, STEP_COUNT as u32); STEP_COUNT]).split(areas[1]);
        for (index, area) in grid.iter().enumerate() {
            let step = &app.project.steps[index]; let is_edit = index == app.edit_cursor; let is_play = app.play_cursor == Some(index);
            let style = match (is_edit, is_play) { (true, true) => Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD), (true, false) => Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD), (false, true) => Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD), _ => Style::default().fg(Color::White) };
            let body = format!("{:02}\n{}\nv{:03}\n{}", index + 1, note_name(step.note), step.velocity, duration_mark(step.duration));
            frame.render_widget(Paragraph::new(body).style(style).block(Block::default().borders(Borders::ALL)), *area);
        }
        let activity = app.activity.iter().cloned().collect::<Vec<_>>().join("\n");
        frame.render_widget(Paragraph::new(activity).block(Block::default().borders(Borders::ALL).title(" MONITOR ACTIVITY ")), areas[2]);
        frame.render_widget(Paragraph::new("←/→ h/l EDIT    ↑/↓ j/k NOTE    J/K OCTAVE    [/] VELOCITY    -/+ GATE    r REST    SPACE PLAY    ,/. BPM    s SAVE    ? HELP    q QUIT"), areas[3]);
    })?;
    Ok(())
}
fn run(dir: PathBuf) -> Result<()> {
    let project = if dir.join("project.toml").exists() {
        load(&dir)?
    } else {
        let project = example_project(&dir);
        save(&project)?;
        project
    };
    let mut app = App::new(project);
    let mut terminal = TerminalGuard::new()?;
    loop {
        app.tick(Instant::now());
        draw(&mut terminal.terminal, &app)?;
        if event::poll(Duration::from_millis(25))? {
            if let Event::Key(key) = event::read()? {
                if app.key(key)? {
                    break;
                }
            }
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("demo") => run(PathBuf::from(
            args.next().unwrap_or_else(|| "examples/first-light".into()),
        )),
        _ => {
            println!("Index demo\n\n  cargo run -- demo examples/first-light");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_wraps() {
        let mut app = App::new(example_project("."));
        app.move_cursor(-1);
        assert_eq!(app.edit_cursor, 15);
        app.move_cursor(1);
        assert_eq!(app.edit_cursor, 0);
    }
    #[test]
    fn transpose_is_bounded() {
        let mut app = App::new(example_project("."));
        app.project.steps[0].note = Some(127);
        app.transpose(12);
        assert_eq!(app.project.steps[0].note, Some(127));
        app.project.steps[0].note = Some(0);
        app.transpose(-12);
        assert_eq!(app.project.steps[0].note, Some(0));
    }
    #[test]
    fn octave_transpose() {
        let mut app = App::new(example_project("."));
        app.transpose(12);
        assert_eq!(app.project.steps[0].note, Some(72));
    }
    #[test]
    fn velocity_bounds() {
        let mut app = App::new(example_project("."));
        app.project.steps[0].velocity = 127;
        app.adjust_velocity(1);
        assert_eq!(app.project.steps[0].velocity, 127);
    }
    #[test]
    fn rest_toggle() {
        let mut app = App::new(example_project("."));
        app.toggle_rest();
        assert_eq!(app.project.steps[0].note, None);
        app.toggle_rest();
        assert_eq!(app.project.steps[0].note, Some(60));
    }
    #[test]
    fn duration_choices() {
        let mut app = App::new(example_project("."));
        app.adjust_duration(1);
        assert_eq!(app.project.steps[0].duration, 0.5);
    }
    #[test]
    fn tempo_bounds() {
        let mut app = App::new(example_project("."));
        app.adjust_tempo(-1000.0);
        assert_eq!(app.project.tempo, 30.0);
        app.adjust_tempo(1000.0);
        assert_eq!(app.project.tempo, 300.0);
    }
    #[test]
    fn playback_cursor_is_independent() {
        let mut app = App::new(example_project("."));
        app.edit_cursor = 7;
        app.toggle_play();
        app.tick(Instant::now());
        assert_eq!(app.edit_cursor, 7);
        assert_eq!(app.play_cursor, Some(1));
    }
}
