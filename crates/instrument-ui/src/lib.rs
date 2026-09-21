use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    AmberCga,
    ConverterBlue,
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color,
    pub structure: Color,
    pub primary: Color,
    pub secondary: Color,
    pub live: Color,
    pub edit: Color,
    pub warning: Color,
    pub error: Color,
}

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::AmberCga => Palette {
            background: Color::Rgb(10, 7, 3),
            structure: Color::Rgb(117, 73, 20),
            primary: Color::Rgb(255, 191, 71),
            secondary: Color::Rgb(190, 143, 58),
            live: Color::Rgb(86, 202, 104),
            edit: Color::Rgb(241, 91, 55),
            warning: Color::Rgb(255, 170, 40),
            error: Color::Rgb(255, 78, 50),
        },
        Theme::ConverterBlue => Palette {
            background: Color::Rgb(3, 10, 20),
            structure: Color::Rgb(34, 112, 159),
            primary: Color::Rgb(238, 248, 255),
            secondary: Color::Rgb(104, 190, 226),
            live: Color::Rgb(86, 221, 125),
            edit: Color::Rgb(237, 75, 65),
            warning: Color::Rgb(255, 188, 57),
            error: Color::Rgb(255, 79, 65),
        },
    }
}

#[derive(Debug, Clone)]
pub struct ShellState {
    pub title: String,
    pub status: String,
    pub command: String,
    pub footer: String,
}

pub struct ShellRegions {
    pub header: Rect,
    pub content: Rect,
    pub command: Rect,
    pub footer: Rect,
}

pub fn shell_regions(area: Rect) -> Option<ShellRegions> {
    if area.width < 60 || area.height < 18 {
        return None;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    Some(ShellRegions {
        header: rows[0],
        content: rows[1],
        command: rows[2],
        footer: rows[3],
    })
}

pub fn render_shell(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    state: &ShellState,
    theme: Theme,
) -> Option<ShellRegions> {
    let regions = shell_regions(area)?;
    let p = palette(theme);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", state.title),
                Style::default().fg(p.primary).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {}", state.status), Style::default().fg(p.live)),
        ]))
        .style(Style::default().bg(p.background))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(p.structure)),
        ),
        regions.header,
    );
    frame.render_widget(
        Paragraph::new(state.command.clone())
            .style(Style::default().fg(p.warning).bg(p.background))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(p.structure)),
            ),
        regions.command,
    );
    frame.render_widget(
        Paragraph::new(state.footer.clone())
            .style(Style::default().fg(p.secondary).bg(p.background)),
        regions.footer,
    );
    Some(regions)
}

pub fn render_minimum(frame: &mut ratatui::Frame<'_>, area: Rect, theme: Theme) {
    let p = palette(theme);
    frame.render_widget(
        Paragraph::new("INDEX\n\nTerminal too small\nMinimum: 60 x 18")
            .style(Style::default().fg(p.warning).bg(p.background))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(p.structure)),
            ),
        area,
    );
}

pub struct Panel<'a> {
    pub title: &'a str,
    pub focused: bool,
    pub body: Text<'a>,
    pub theme: Theme,
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = palette(self.theme);
        let color = if self.focused { p.edit } else { p.structure };
        Paragraph::new(self.body)
            .style(Style::default().fg(p.primary).bg(p.background))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
                    .title(Span::styled(
                        format!(" {} ", self.title.to_uppercase()),
                        Style::default()
                            .fg(if self.focused { p.edit } else { p.secondary })
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .render(area, buf);
    }
}

#[derive(Debug, Clone)]
pub struct SequenceStep {
    pub number: usize,
    pub note: String,
    pub velocity: u8,
    pub duration: String,
}

pub struct SequenceGrid<'a> {
    pub steps: &'a [SequenceStep],
    pub edit_cursor: usize,
    pub playhead: Option<usize>,
    pub theme: Theme,
}

impl Widget for SequenceGrid<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = palette(self.theme);
        let two_rows = area.width < 120;
        let cols = if two_rows { 8 } else { 16 };
        let rows = if two_rows { 2 } else { 1 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                (0..rows)
                    .map(|_| Constraint::Ratio(1, rows as u32))
                    .collect::<Vec<_>>(),
            )
            .split(area);
        for row in 0..rows {
            let cells = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    (0..cols)
                        .map(|_| Constraint::Ratio(1, cols as u32))
                        .collect::<Vec<_>>(),
                )
                .split(chunks[row]);
            for col in 0..cols {
                let index = row * cols + col;
                if let Some(step) = self.steps.get(index) {
                    let both = self.edit_cursor == index && self.playhead == Some(index);
                    let style = if both {
                        Style::default()
                            .fg(Color::Black)
                            .bg(p.warning)
                            .add_modifier(Modifier::BOLD)
                    } else if self.edit_cursor == index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(p.edit)
                            .add_modifier(Modifier::BOLD)
                    } else if self.playhead == Some(index) {
                        Style::default()
                            .fg(Color::Black)
                            .bg(p.live)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(p.primary).bg(p.background)
                    };
                    let text = format!(
                        "{:02}\n{}\nv{:03}\n{}",
                        step.number, step.note, step.velocity, step.duration
                    );
                    Paragraph::new(text)
                        .style(style)
                        .block(Block::default().borders(Borders::ALL).border_style(
                            Style::default().fg(if self.edit_cursor == index {
                                p.edit
                            } else {
                                p.structure
                            }),
                        ))
                        .render(cells[col], buf);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamState {
    Normal,
    Focused,
    Active,
    Disabled,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ParameterRow {
    pub label: String,
    pub value: String,
    pub state: ParamState,
}

pub struct ParameterList<'a> {
    pub rows: &'a [ParameterRow],
    pub theme: Theme,
}

impl Widget for ParameterList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = palette(self.theme);
        let lines = self
            .rows
            .iter()
            .map(|row| {
                let color = match row.state {
                    ParamState::Focused => p.edit,
                    ParamState::Active => p.live,
                    ParamState::Disabled => p.secondary,
                    ParamState::Warning => p.warning,
                    ParamState::Normal => p.primary,
                };
                Line::from(vec![
                    Span::styled(
                        format!("{:11} ", row.label.to_uppercase()),
                        Style::default().fg(p.secondary),
                    ),
                    Span::styled(
                        row.value.clone(),
                        Style::default().fg(color).add_modifier(
                            if row.state == ParamState::Focused {
                                Modifier::BOLD | Modifier::UNDERLINED
                            } else {
                                Modifier::empty()
                            },
                        ),
                    ),
                ])
            })
            .collect::<Vec<_>>();
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(p.structure))
                    .title(Span::styled(
                        " PARAMETERS ",
                        Style::default()
                            .fg(p.secondary)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .render(area, buf);
    }
}

#[derive(Debug, Clone)]
pub struct ScopeState {
    pub motion: Vec<f64>,
    pub values: Vec<f64>,
    pub meter: f64,
    pub peak: f64,
    pub playhead: Option<usize>,
}

pub struct Scope<'a> {
    pub state: &'a ScopeState,
    pub theme: Theme,
}

impl Widget for Scope<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = palette(self.theme);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.structure))
            .title(Span::styled(
                " SEQUENCE SCOPE ",
                Style::default()
                    .fg(p.secondary)
                    .add_modifier(Modifier::BOLD),
            ));
        let plot = block.inner(area);
        block.render(area, buf);
        if plot.width < 2 || plot.height < 2 {
            return;
        }
        plot_values(self.state, plot, buf, p);
    }
}

fn plot_values(state: &ScopeState, area: Rect, buf: &mut Buffer, p: Palette) {
    let data = if state.motion.is_empty() {
        &state.values
    } else {
        &state.motion
    };
    if data.is_empty() {
        return;
    }
    let width = area.width as usize;
    let height = area.height as usize;
    for x in 0..width {
        let source = x * data.len() / width;
        let value = data[source.min(data.len() - 1)].clamp(-1.0, 1.0);
        let y = ((1.0 - (value + 1.0) / 2.0) * height.saturating_sub(1) as f64) as u16;
        let cell = area.x + x as u16;
        let row = area.y + y.min(area.height - 1);
        buf[(cell, row)].set_char('⣿').set_fg(p.live);
    }
    if let Some(playhead) = state.playhead {
        let x = area.x + (playhead as u16).min(area.width - 1);
        for y in area.y..area.bottom() {
            buf[(x, y)].set_char('│').set_fg(p.warning);
        }
    }
    let meter = ((state.meter.clamp(0.0, 1.0) * area.width as f64) as u16).min(area.width);
    for x in 0..meter {
        buf[(area.x + x, area.bottom() - 1)]
            .set_char('━')
            .set_fg(p.live);
    }
    let peak =
        ((state.peak.clamp(0.0, 1.0) * area.width as f64) as u16).min(area.width.saturating_sub(1));
    buf[(area.x + peak, area.bottom() - 1)]
        .set_char('┫')
        .set_fg(p.warning);
}

#[derive(Debug, Clone)]
pub struct ModRoute {
    pub source: String,
    pub destination: String,
    pub depth: String,
    pub active: bool,
    pub spark: Vec<f64>,
}

pub struct ModulationMatrix<'a> {
    pub routes: &'a [ModRoute],
    pub theme: Theme,
}

impl Widget for ModulationMatrix<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = palette(self.theme);
        let mut lines = vec![Line::from(Span::styled(
            "SOURCE       DEST         DEPTH",
            Style::default().fg(p.secondary),
        ))];
        for route in self.routes {
            let mark = if route.active { "●" } else { "○" };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "{} {:10} {:11} {:>5} ",
                        mark, route.source, route.destination, route.depth
                    ),
                    Style::default().fg(if route.active { p.primary } else { p.secondary }),
                ),
                Span::styled(
                    sparkline(&route.spark),
                    Style::default().fg(if route.active { p.live } else { p.secondary }),
                ),
            ]));
        }
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(p.structure))
                    .title(Span::styled(
                        " MODULATION ",
                        Style::default()
                            .fg(p.secondary)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .render(area, buf);
    }
}

pub fn sparkline(values: &[f64]) -> String {
    const GLYPHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    values
        .iter()
        .map(|value| GLYPHS[((value.clamp(0.0, 1.0) * 7.0).round() as usize).min(7)])
        .collect()
}

#[derive(Debug, Clone)]
pub enum EditorValue {
    Enum {
        options: Vec<String>,
        selected: usize,
    },
    Number {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    Toggle(bool),
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub title: String,
    pub label: String,
    pub value: EditorValue,
}

pub struct Popdown<'a> {
    pub editor: &'a EditorState,
    pub theme: Theme,
}

impl Widget for Popdown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = palette(self.theme);
        let width = area.width.min(48);
        let height = 7.min(area.height);
        let rect = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        Clear.render(rect, buf);
        let value = match &self.editor.value {
            EditorValue::Enum { options, selected } => {
                options.get(*selected).cloned().unwrap_or_default()
            }
            EditorValue::Number { value, .. } => format!("{value:.2}"),
            EditorValue::Toggle(value) => {
                if *value {
                    "ON".into()
                } else {
                    "OFF".into()
                }
            }
        };
        Paragraph::new(vec![
            Line::from(self.editor.label.clone()),
            Line::from(Span::styled(
                value,
                Style::default().fg(p.primary).add_modifier(Modifier::BOLD),
            )),
            Line::from("←/→ adjust   Enter accept   Esc cancel"),
        ])
        .style(Style::default().fg(p.primary).bg(p.background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.edit))
                .title(Span::styled(
                    format!(" {} ", self.editor.title),
                    Style::default().fg(p.edit).add_modifier(Modifier::BOLD),
                )),
        )
        .render(rect, buf);
    }
}

pub struct HelpOverlay {
    pub theme: Theme,
}

impl Widget for HelpOverlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = palette(self.theme);
        let width = area.width.min(64);
        let height = area.height.min(15);
        let rect = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        Clear.render(rect, buf);
        Paragraph::new(
            "NAVIGATION\n  ←/→ h/l     move edit cursor\n  ↑/↓ j/k     change note\n  J/K          octave\n\nEDIT\n  [ ]          velocity\n  - +          duration\n  r             rest\n  Space         play / stop\n  t             theme\n  Enter         parameter editor\n  Esc           close overlay\n  q             quit",
        )
        .style(Style::default().fg(p.primary).bg(p.background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.edit))
                .title(Span::styled(
                    " HELP ",
                    Style::default().fg(p.edit).add_modifier(Modifier::BOLD),
                )),
        )
        .render(rect, buf);
    }
}

#[derive(Debug, Clone)]
pub struct InstrumentView {
    pub shell: ShellState,
    pub sequence: SequenceGridState,
    pub parameters: Vec<ParameterRow>,
    pub scope: ScopeState,
    pub modulation: Vec<ModRoute>,
    pub theme: Theme,
    pub editor: Option<EditorState>,
    pub help: bool,
}

#[derive(Debug, Clone)]
pub struct SequenceGridState {
    pub steps: Vec<SequenceStep>,
    pub edit_cursor: usize,
    pub playhead: Option<usize>,
}

pub fn render_instrument(frame: &mut ratatui::Frame<'_>, area: Rect, view: &InstrumentView) {
    let Some(regions) = render_shell(frame, area, &view.shell, view.theme) else {
        render_minimum(frame, area, view.theme);
        return;
    };
    let p = palette(view.theme);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(regions.content);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(6)])
        .split(columns[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(6)])
        .split(columns[1]);
    frame.render_widget(
        Panel {
            title: "PATTERN / 16 STEPS",
            focused: true,
            body: Text::raw(""),
            theme: view.theme,
        },
        left[0],
    );
    let inner = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.edit))
        .inner(left[0]);
    frame.render_widget(
        SequenceGrid {
            steps: &view.sequence.steps,
            edit_cursor: view.sequence.edit_cursor,
            playhead: view.sequence.playhead,
            theme: view.theme,
        },
        inner,
    );
    frame.render_widget(
        Scope {
            state: &view.scope,
            theme: view.theme,
        },
        left[1],
    );
    frame.render_widget(
        ParameterList {
            rows: &view.parameters,
            theme: view.theme,
        },
        right[0],
    );
    frame.render_widget(
        ModulationMatrix {
            routes: &view.modulation,
            theme: view.theme,
        },
        right[1],
    );
    if let Some(editor) = &view.editor {
        frame.render_widget(
            Popdown {
                editor,
                theme: view.theme,
            },
            area,
        );
    }
    if view.help {
        frame.render_widget(HelpOverlay { theme: view.theme }, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn screen(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn view(theme: Theme) -> InstrumentView {
        InstrumentView {
            shell: ShellState {
                title: "FIRST LIGHT".into(),
                status: "PLAYING".into(),
                command: "EVENT".into(),
                footer: "F1 PERFORM".into(),
            },
            sequence: SequenceGridState {
                steps: (1..=16)
                    .map(|number| SequenceStep {
                        number,
                        note: "C4".into(),
                        velocity: 100,
                        duration: "1/4".into(),
                    })
                    .collect(),
                edit_cursor: 5,
                playhead: Some(5),
            },
            parameters: vec![ParameterRow {
                label: "pitch".into(),
                value: "C4".into(),
                state: ParamState::Focused,
            }],
            scope: ScopeState {
                motion: vec![0.0, 0.5, -0.2, 0.8],
                values: vec![],
                meter: 0.7,
                peak: 0.9,
                playhead: Some(3),
            },
            modulation: vec![ModRoute {
                source: "LFO".into(),
                destination: "PITCH".into(),
                depth: "+12 st".into(),
                active: true,
                spark: vec![0.1, 0.5, 0.8],
            }],
            theme,
            editor: None,
            help: false,
        }
    }

    #[test]
    fn renders_both_themes_and_sizes() {
        for theme in [Theme::AmberCga, Theme::ConverterBlue] {
            for (width, height) in [(120, 40), (80, 30)] {
                let backend = TestBackend::new(width, height);
                let mut terminal = Terminal::new(backend).unwrap();
                terminal
                    .draw(|frame| render_instrument(frame, frame.area(), &view(theme)))
                    .unwrap();
                assert!(screen(&terminal).contains("FIRST LIGHT"));
            }
        }
    }

    #[test]
    fn small_terminal_falls_back() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_instrument(frame, frame.area(), &view(Theme::AmberCga)))
            .unwrap();
        assert!(screen(&terminal).contains("too small"));
    }

    #[test]
    fn overlap_and_overlays_render() {
        let mut view = view(Theme::ConverterBlue);
        view.editor = Some(EditorState {
            title: "PITCH".into(),
            label: "Pitch".into(),
            value: EditorValue::Number {
                value: 60.0,
                min: 0.0,
                max: 127.0,
                step: 1.0,
            },
        });
        view.help = true;
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_instrument(frame, frame.area(), &view))
            .unwrap();
        assert!(screen(&terminal).contains("HELP"));
        assert!(screen(&terminal).contains("PITCH"));
    }
}
