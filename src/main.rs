use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use instrument_ui::{
    render_instrument, EditorState, EditorValue, InstrumentView, ModRoute, ParamState,
    ParameterRow, ScopeState, SequenceGridState, SequenceStep, ShellState, Theme,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    collections::VecDeque,
    env, io,
    time::{Duration, Instant},
};

const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const DURATIONS: [f64; 4] = [0.125, 0.25, 0.5, 1.0];

struct Demo {
    notes: Vec<Option<i16>>,
    velocities: Vec<u8>,
    durations: Vec<f64>,
    tempo: f64,
    edit_cursor: usize,
    playhead: usize,
    playing: bool,
    next_step: Instant,
    theme: Theme,
    help: bool,
    editor: Option<EditorState>,
    activity: VecDeque<String>,
    phase: f64,
}

impl Demo {
    fn new() -> Self {
        Self {
            notes: vec![
                Some(60),
                Some(60),
                Some(63),
                Some(65),
                Some(63),
                Some(60),
                Some(58),
                Some(55),
                Some(60),
                Some(67),
                Some(65),
                Some(63),
                Some(60),
                Some(58),
                Some(55),
                Some(53),
            ],
            velocities: vec![
                112, 92, 106, 98, 90, 108, 94, 84, 106, 96, 114, 100, 88, 102, 92, 80,
            ],
            durations: vec![0.25; 16],
            tempo: 124.0,
            edit_cursor: 5,
            playhead: 0,
            playing: true,
            next_step: Instant::now(),
            theme: Theme::AmberCga,
            help: false,
            editor: None,
            activity: VecDeque::new(),
            phase: 0.0,
        }
    }

    fn note_name(note: Option<i16>) -> String {
        let Some(note) = note else {
            return "REST".into();
        };
        format!(
            "{}{}",
            NOTES[note.rem_euclid(12) as usize],
            note.div_euclid(12) - 1
        )
    }

    fn step_views(&self) -> Vec<SequenceStep> {
        self.notes
            .iter()
            .enumerate()
            .map(|(index, note)| SequenceStep {
                number: index + 1,
                note: Self::note_name(*note),
                velocity: self.velocities[index],
                duration: match self.durations[index] {
                    x if (x - 0.125).abs() < f64::EPSILON => "1/8".into(),
                    x if (x - 0.25).abs() < f64::EPSILON => "1/4".into(),
                    x if (x - 0.5).abs() < f64::EPSILON => "1/2".into(),
                    _ => "1".into(),
                },
            })
            .collect()
    }

    fn view(&self) -> InstrumentView {
        let current = self.edit_cursor;
        let params = vec![
            ParameterRow {
                label: "pitch".into(),
                value: Self::note_name(self.notes[current]),
                state: ParamState::Focused,
            },
            ParameterRow {
                label: "velocity".into(),
                value: self.velocities[current].to_string(),
                state: ParamState::Active,
            },
            ParameterRow {
                label: "duration".into(),
                value: format!("{:.2} beat", self.durations[current]),
                state: ParamState::Normal,
            },
            ParameterRow {
                label: "gate".into(),
                value: "0.80".into(),
                state: ParamState::Normal,
            },
            ParameterRow {
                label: "probability".into(),
                value: "100%".into(),
                state: ParamState::Normal,
            },
            ParameterRow {
                label: "ratchet".into(),
                value: "1x".into(),
                state: ParamState::Normal,
            },
            ParameterRow {
                label: "condition".into(),
                value: "1:1".into(),
                state: ParamState::Normal,
            },
            ParameterRow {
                label: "modulation".into(),
                value: "LFO > PITCH".into(),
                state: ParamState::Warning,
            },
        ];
        let motion = (0..96)
            .map(|i| (self.phase + i as f64 * 0.16).sin() * 0.7)
            .collect();
        InstrumentView {
            shell: ShellState {
                title: "INDEX // FIRST LIGHT".into(),
                status: if self.playing {
                    "PLAYING  •  MONITOR"
                } else {
                    "STOPPED  •  MONITOR"
                }
                .into(),
                command: format!(
                    "EVENT  STEP {:02}  {}  |  {}",
                    self.playhead + 1,
                    Self::note_name(self.notes[self.playhead]),
                    self.activity
                        .back()
                        .cloned()
                        .unwrap_or_else(|| "READY".into())
                ),
                footer:
                    "F1 PERFORM   F2 SEQUENCE   F3 DEVICES   F4 LOG       t THEME   ? HELP   q QUIT"
                        .into(),
            },
            sequence: SequenceGridState {
                steps: self.step_views(),
                edit_cursor: self.edit_cursor,
                playhead: self.playing.then_some(self.playhead),
            },
            parameters: params,
            scope: ScopeState {
                motion,
                values: vec![],
                meter: 0.62 + self.phase.sin() * 0.2,
                peak: 0.86,
                playhead: Some((self.playhead * 5).min(95)),
            },
            modulation: vec![
                ModRoute {
                    source: "LFO 1".into(),
                    destination: "PITCH".into(),
                    depth: "+07 st".into(),
                    active: true,
                    spark: vec![0.2, 0.4, 0.8, 0.6, 0.9],
                },
                ModRoute {
                    source: "ENV".into(),
                    destination: "VELOCITY".into(),
                    depth: "+18".into(),
                    active: true,
                    spark: vec![0.1, 0.6, 0.4, 0.8],
                },
                ModRoute {
                    source: "RND".into(),
                    destination: "DURATION".into(),
                    depth: "-12%".into(),
                    active: false,
                    spark: vec![0.5, 0.4, 0.5],
                },
                ModRoute {
                    source: "STEP".into(),
                    destination: "RATCHET".into(),
                    depth: "+01".into(),
                    active: true,
                    spark: vec![0.8, 0.7, 0.9, 0.8],
                },
            ],
            theme: self.theme,
            editor: self.editor.clone(),
            help: self.help,
        }
    }

    fn tick(&mut self, now: Instant) {
        self.phase += 0.08;
        if !self.playing {
            return;
        }
        while now >= self.next_step {
            if let Some(note) = self.notes[self.playhead] {
                push_log(
                    &mut self.activity,
                    format!(
                        "MONITOR  {:02}  ON  {}  v{}",
                        self.playhead + 1,
                        Self::note_name(Some(note)),
                        self.velocities[self.playhead]
                    ),
                );
            } else {
                push_log(
                    &mut self.activity,
                    format!("MONITOR  {:02}  REST", self.playhead + 1),
                );
            }
            self.next_step += Duration::from_secs_f64(
                (60.0 / self.tempo * self.durations[self.playhead]).max(0.01),
            );
            self.playhead = (self.playhead + 1) % 16;
        }
    }

    fn change_note(&mut self, amount: i16) {
        let note = self.notes[self.edit_cursor].unwrap_or(60);
        self.notes[self.edit_cursor] = Some((note + amount).clamp(0, 127));
    }

    fn adjust_editor(&mut self, amount: i16) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        match &mut editor.value {
            EditorValue::Number {
                value,
                min,
                max,
                step,
            } => *value = (*value + *step * amount as f64).clamp(*min, *max),
            EditorValue::Toggle(value) => {
                if amount != 0 {
                    *value = !*value
                }
            }
            EditorValue::Enum { options, selected } => {
                *selected = (*selected as i16 + amount).rem_euclid(options.len() as i16) as usize
            }
        }
    }

    fn key(&mut self, key: KeyEvent) -> bool {
        if self.help {
            if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') {
                self.help = false;
            }
            return false;
        }
        if self.editor.is_some() {
            match key.code {
                KeyCode::Esc => self.editor = None,
                KeyCode::Left | KeyCode::Char('h') => self.adjust_editor(-1),
                KeyCode::Right | KeyCode::Char('l') => self.adjust_editor(1),
                KeyCode::Enter => self.editor = None,
                _ => {}
            }
            return false;
        }
        let octave = key.modifiers.contains(KeyModifiers::SHIFT)
            || matches!(key.code, KeyCode::Char('J') | KeyCode::Char('K'));
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Left | KeyCode::Char('h') => self.edit_cursor = (self.edit_cursor + 15) % 16,
            KeyCode::Right | KeyCode::Char('l') => self.edit_cursor = (self.edit_cursor + 1) % 16,
            KeyCode::Up | KeyCode::Char('j') | KeyCode::Char('J') if octave => self.change_note(12),
            KeyCode::Down | KeyCode::Char('k') | KeyCode::Char('K') if octave => {
                self.change_note(-12)
            }
            KeyCode::Up | KeyCode::Char('j') => self.change_note(1),
            KeyCode::Down | KeyCode::Char('k') => self.change_note(-1),
            KeyCode::Char('[') => {
                self.velocities[self.edit_cursor] =
                    self.velocities[self.edit_cursor].saturating_sub(1)
            }
            KeyCode::Char(']') => {
                self.velocities[self.edit_cursor] = (self.velocities[self.edit_cursor] + 1).min(127)
            }
            KeyCode::Char('-') | KeyCode::Char('+') => {
                let current = DURATIONS
                    .iter()
                    .position(|x| (*x - self.durations[self.edit_cursor]).abs() < f64::EPSILON)
                    .unwrap_or(1);
                let delta = if key.code == KeyCode::Char('-') {
                    -1
                } else {
                    1
                };
                self.durations[self.edit_cursor] =
                    DURATIONS[(current as i16 + delta).clamp(0, 3) as usize];
            }
            KeyCode::Char('r') => {
                self.notes[self.edit_cursor] = if self.notes[self.edit_cursor].is_some() {
                    None
                } else {
                    Some(60)
                }
            }
            KeyCode::Char(' ') => {
                self.playing = !self.playing;
                if self.playing {
                    self.next_step = Instant::now();
                }
            }
            KeyCode::Char('t') => {
                self.theme = if self.theme == Theme::AmberCga {
                    Theme::ConverterBlue
                } else {
                    Theme::AmberCga
                }
            }
            KeyCode::Enter => {
                self.editor = Some(EditorState {
                    title: "GATE".into(),
                    label: "Gate amount".into(),
                    value: EditorValue::Number {
                        value: 0.8,
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                })
            }
            KeyCode::Char('?') => self.help = true,
            _ => {}
        }
        false
    }
}

fn push_log(log: &mut VecDeque<String>, value: String) {
    if log.len() == 5 {
        log.pop_front();
    }
    log.push_back(value);
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}
impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(stdout))?,
        })
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn run_ui_demo() -> Result<()> {
    let mut demo = Demo::new();
    let mut terminal = TerminalGuard::new()?;
    loop {
        demo.tick(Instant::now());
        terminal
            .terminal
            .draw(|frame| render_instrument(frame, frame.area(), &demo.view()))?;
        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                if demo.key(key) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    match env::args().nth(1).as_deref() {
        Some("ui-demo") => run_ui_demo(),
        _ => {
            println!("Index UI demo\n\n  cargo run -- ui-demo");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_switching_is_local_to_demo() {
        let mut demo = Demo::new();
        assert_eq!(demo.theme, Theme::AmberCga);
        assert!(!demo.key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)));
        assert_eq!(demo.theme, Theme::ConverterBlue);
    }

    #[test]
    fn demo_starts_with_requested_visual_state() {
        let demo = Demo::new();
        assert_eq!(demo.edit_cursor, 5);
        assert_eq!(demo.tempo, 124.0);
        assert!(demo.playing);
        assert_eq!(demo.notes.len(), 16);
    }
}
