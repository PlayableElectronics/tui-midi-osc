use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use instrument_ui::{render, DemoView, Theme};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, io, time::Duration};

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        let terminal =
            Terminal::new(CrosstermBackend::new(stdout)).context("initialize terminal")?;
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

fn run_ui_demo() -> Result<()> {
    let view = DemoView::canonical();
    let mut theme = Theme::AmberCga;
    let mut terminal = TerminalGuard::new()?;
    loop {
        terminal
            .terminal
            .draw(|frame| render(frame, frame.area(), &view, theme))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('t') => {
                        theme = if theme == Theme::AmberCga {
                            Theme::ConverterBlue
                        } else {
                            Theme::AmberCga
                        }
                    }
                    _ => {}
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
