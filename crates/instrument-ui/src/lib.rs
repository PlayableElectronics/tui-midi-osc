use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub const STANDARD_SIZE: (u16, u16) = (120, 40);
pub const COMPACT_SIZE: (u16, u16) = (80, 30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    AmberCga,
    ConverterBlue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub background: Color,
    pub panel: Color,
    pub structure_dim: Color,
    pub structure: Color,
    pub primary: Color,
    pub value: Color,
    pub edit: Color,
    pub live: Color,
    pub error: Color,
}

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::AmberCga => Palette {
            background: Color::Rgb(0x12, 0x09, 0x00),
            panel: Color::Rgb(0x08, 0x03, 0x00),
            structure_dim: Color::Rgb(0x8a, 0x57, 0x00),
            structure: Color::Rgb(0xd7, 0x8a, 0x00),
            primary: Color::Rgb(0xff, 0xb0, 0x00),
            value: Color::Rgb(0xff, 0xe1, 0xa0),
            edit: Color::Rgb(0xe8, 0x4a, 0x1a),
            live: Color::Rgb(0x63, 0xd8, 0x6b),
            error: Color::Rgb(0xff, 0x3b, 0x30),
        },
        Theme::ConverterBlue => Palette {
            background: Color::Rgb(0x02, 0x07, 0x2a),
            panel: Color::Rgb(0x00, 0x03, 0x17),
            structure_dim: Color::Rgb(0x16, 0x5d, 0x82),
            structure: Color::Rgb(0x00, 0xb8, 0xd9),
            primary: Color::Rgb(0x00, 0xd9, 0xff),
            value: Color::Rgb(0xe8, 0xfb, 0xff),
            edit: Color::Rgb(0xff, 0x31, 0x5f),
            live: Color::Rgb(0x36, 0xff, 0x77),
            error: Color::Rgb(0xff, 0x31, 0x5f),
        },
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepView {
    pub number: usize,
    pub pitch: String,
    pub velocity: u8,
    pub duration: String,
    pub flags: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterView {
    pub label: String,
    pub value: String,
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteView {
    pub source: String,
    pub destination: String,
    pub amount: String,
    pub level: u8,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DemoView {
    pub project: String,
    pub transport: String,
    pub tempo: String,
    pub clock_division: String,
    pub bar: String,
    pub midi_ready: bool,
    pub osc_ready: bool,
    pub sc_state: String,
    pub steps: Vec<StepView>,
    pub edit_cursor: usize,
    pub playhead: usize,
    pub parameters: Vec<ParameterView>,
    pub pitch_trace: Vec<i16>,
    pub velocity_trace: Vec<u8>,
    pub routes: Vec<RouteView>,
    pub context_label: String,
    pub source_lines: Vec<String>,
    pub staged: bool,
}

impl DemoView {
    pub fn canonical() -> Self {
        let pitches = [
            ("C4", 60, 104),
            ("C4", 60, 92),
            ("D#4", 63, 110),
            ("F4", 65, 86),
            ("D#4", 63, 108),
            ("C4", 60, 116),
            ("A#3", 58, 78),
            ("G3", 55, 91),
            ("C4", 60, 96),
            ("REST", 60, 0),
            ("G4", 67, 72),
            ("D4", 62, 100),
            ("F4", 65, 88),
            ("C4", 60, 115),
            ("D#4", 63, 84),
            ("REST", 60, 0),
        ];
        let steps = pitches
            .iter()
            .enumerate()
            .map(|(index, (pitch, _, velocity))| StepView {
                number: index + 1,
                pitch: (*pitch).into(),
                velocity: *velocity,
                duration: "1/16".into(),
                flags: if index == 4 {
                    "EDIT".into()
                } else if index == 5 {
                    "LIVE".into()
                } else {
                    "—".into()
                },
            })
            .collect();
        Self {
            project: "INDEX // FIRST LIGHT".into(),
            transport: "PLAYING".into(),
            tempo: "132.0 BPM".into(),
            clock_division: "1/16".into(),
            bar: "BAR 017.03".into(),
            midi_ready: true,
            osc_ready: true,
            sc_state: "SC READY".into(),
            steps,
            edit_cursor: 4,
            playhead: 5,
            parameters: vec![
                parameter("STEP", "05 / 16", false),
                parameter("PITCH", "D#4", true),
                parameter("VELOCITY", "108", false),
                parameter("DURATION", "0.25 beat", false),
                parameter("GATE", "0.80", false),
                parameter("PROBABILITY", "100%", false),
                parameter("RATCHET", "1×", false),
                parameter("CONDITION", "1:1", false),
                parameter("OUTPUT", "nerdseq.cv1", false),
                parameter("CHANNEL", "MIDI 01", false),
                parameter("FOCUS", "pitch", false),
                parameter("MOD", "lfo_1 +7st", false),
            ],
            pitch_trace: pitches.iter().map(|(_, pitch, _)| *pitch).collect(),
            velocity_trace: pitches.iter().map(|(_, _, velocity)| *velocity).collect(),
            routes: vec![
                RouteView {
                    source: "lfo_1".into(),
                    destination: "pitch".into(),
                    amount: "+07 st".into(),
                    level: 7,
                    active: true,
                },
                RouteView {
                    source: "env_a".into(),
                    destination: "velocity".into(),
                    amount: "+18".into(),
                    level: 4,
                    active: true,
                },
            ],
            context_label: "STEP 05 / pitch".into(),
            source_lines: vec![
                "Pbind(\\degree, Pseq([0, 0, 3, 5, 3, 0, -2, -5], inf),".into(),
                "      \\dur, 0.25, \\amp, Pkey(\\velocity) / 127)".into(),
            ],
            staged: true,
        }
    }
}

fn parameter(label: &str, value: &str, focused: bool) -> ParameterView {
    ParameterView {
        label: label.into(),
        value: value.into(),
        focused,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Standard,
    Compact,
    TooSmall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regions {
    pub mode: LayoutMode,
    pub status: Rect,
    pub pattern: Rect,
    pub parameters: Rect,
    pub scope: Rect,
    pub modulation: Option<Rect>,
    pub context: Rect,
    pub commands: Rect,
}

pub fn regions(area: Rect) -> Regions {
    if area.width >= STANDARD_SIZE.0 && area.height >= STANDARD_SIZE.1 {
        Regions {
            mode: LayoutMode::Standard,
            status: rect(area, 0, 0, 120, 2),
            pattern: rect(area, 0, 2, 84, 10),
            parameters: rect(area, 84, 2, 36, 19),
            scope: rect(area, 0, 12, 84, 19),
            modulation: Some(rect(area, 84, 21, 36, 10)),
            context: rect(area, 0, 31, 120, 6),
            commands: rect(area, 0, 37, 120, 3),
        }
    } else if area.width >= COMPACT_SIZE.0 && area.height >= COMPACT_SIZE.1 {
        Regions {
            mode: LayoutMode::Compact,
            status: rect(area, 0, 0, 80, 2),
            pattern: rect(area, 0, 2, 80, 10),
            parameters: rect(area, 51, 12, 29, 12),
            scope: rect(area, 0, 12, 51, 12),
            modulation: None,
            context: rect(area, 0, 24, 80, 3),
            commands: rect(area, 0, 27, 80, 3),
        }
    } else {
        Regions {
            mode: LayoutMode::TooSmall,
            status: area,
            pattern: Rect::default(),
            parameters: Rect::default(),
            scope: Rect::default(),
            modulation: None,
            context: Rect::default(),
            commands: Rect::default(),
        }
    }
}

fn rect(area: Rect, x: u16, y: u16, width: u16, height: u16) -> Rect {
    Rect::new(area.x + x, area.y + y, width, height)
}

pub fn render(frame: &mut ratatui::Frame<'_>, area: Rect, view: &DemoView, theme: Theme) {
    let palette = palette(theme);
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.background)),
        area,
    );
    let regions = regions(area);
    if regions.mode == LayoutMode::TooSmall {
        render_too_small(frame, area, palette);
        return;
    }
    render_status(frame, regions.status, view, palette, regions.mode);
    render_pattern(frame, regions.pattern, view, palette);
    render_parameters(frame, regions.parameters, view, palette, regions.mode);
    render_scope(frame, regions.scope, view, palette);
    if let Some(area) = regions.modulation {
        render_modulation(frame, area, view, palette);
    }
    render_context(frame, regions.context, view, palette, regions.mode);
    render_commands(frame, regions.commands, palette, regions.mode);
}

fn panel(title: &str, palette: Palette, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(if focused {
            palette.structure
        } else {
            palette.structure_dim
        }))
        .style(Style::default().bg(palette.panel))
        .title(Span::styled(
            format!("┤ {title} ├"),
            Style::default()
                .fg(palette.structure)
                .add_modifier(Modifier::BOLD),
        ))
}

fn render_status(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    view: &DemoView,
    palette: Palette,
    mode: LayoutMode,
) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(palette.structure))
        .style(Style::default().bg(palette.panel));
    let line = if mode == LayoutMode::Standard {
        Line::from(vec![
            Span::styled(
                format!(" {} ", view.project),
                Style::default()
                    .fg(palette.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}  ", view.transport),
                Style::default()
                    .fg(palette.live)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{}   {}   {}          ",
                    view.tempo, view.clock_division, view.bar
                ),
                Style::default().fg(palette.primary),
            ),
            live_indicator("MIDI", view.midi_ready, palette),
            Span::raw("  "),
            live_indicator("OSC", view.osc_ready, palette),
            Span::styled(
                format!("  {}", view.sc_state),
                Style::default().fg(palette.value),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                " INDEX // FIRST LIGHT ",
                Style::default()
                    .fg(palette.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " PLAYING ",
                Style::default()
                    .fg(palette.live)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}  {}  ", view.tempo, view.bar),
                Style::default().fg(palette.primary),
            ),
            live_indicator("I/O", view.midi_ready && view.osc_ready, palette),
        ])
    };
    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn live_indicator(label: &str, ready: bool, palette: Palette) -> Span<'static> {
    Span::styled(
        format!("{label} {}", if ready { "●" } else { "○" }),
        Style::default().fg(if ready {
            palette.live
        } else {
            palette.structure_dim
        }),
    )
}

fn render_pattern(frame: &mut ratatui::Frame<'_>, area: Rect, view: &DemoView, palette: Palette) {
    let block = panel("PATTERN 01 — 16 STEPS", palette, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let cell_width = if inner.width >= 82 { 9 } else { 8 };
    let row_height = 3;
    for index in 0..16 {
        let row = index / 8;
        let column = index % 8;
        let x = inner.x + 1 + column as u16 * (cell_width + 1);
        let y = inner.y + row as u16 * (row_height + 1);
        let width = cell_width.min(inner.right().saturating_sub(x));
        let height = row_height.min(inner.bottom().saturating_sub(y));
        if width == 0 || height == 0 {
            continue;
        }
        render_step(
            frame,
            Rect::new(x, y, width, height),
            &view.steps[index],
            index == view.edit_cursor,
            index == view.playhead,
            palette,
        );
    }
}

fn render_step(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    step: &StepView,
    selected: bool,
    live: bool,
    palette: Palette,
) {
    let both = selected && live;
    let background = if live {
        palette.live
    } else if selected {
        palette.edit
    } else {
        palette.panel
    };
    let foreground = if both {
        palette.edit
    } else if live {
        palette.background
    } else {
        palette.value
    };
    let border = if live {
        palette.live
    } else if selected {
        palette.structure
    } else {
        palette.structure_dim
    };
    let header = if selected {
        format!("{:02} EDIT", step.number)
    } else if live {
        format!("{:02} LIVE", step.number)
    } else {
        format!("{:02}", step.number)
    };
    let body = if step.pitch == "REST" {
        "REST".to_string()
    } else {
        format!("{} {:>3}", step.pitch, step.velocity)
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                header,
                Style::default().fg(if selected {
                    palette.value
                } else if live {
                    palette.background
                } else {
                    palette.structure_dim
                }),
            )),
            Line::from(Span::styled(body, Style::default().fg(foreground))),
            Line::from(Span::styled(
                format!("{} {}", step.duration, step.flags),
                Style::default().fg(if live {
                    palette.background
                } else {
                    palette.primary
                }),
            )),
        ])
        .style(Style::default().fg(foreground).bg(background)),
        area,
    );
    for y in area.y..area.bottom() {
        frame.buffer_mut()[(area.right() - 1, y)]
            .set_char('│')
            .set_fg(border)
            .set_bg(background);
    }
}

fn render_parameters(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    view: &DemoView,
    palette: Palette,
    mode: LayoutMode,
) {
    let block = panel("PARAMETERS", palette, false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let visible = if mode == LayoutMode::Standard { 12 } else { 9 };
    let label_width = if mode == LayoutMode::Standard { 14 } else { 11 };
    let lines = view
        .parameters
        .iter()
        .take(visible)
        .map(|parameter| {
            let value = Span::styled(
                parameter.value.clone(),
                Style::default()
                    .fg(palette.value)
                    .bg(if parameter.focused {
                        palette.edit
                    } else {
                        palette.panel
                    })
                    .add_modifier(if parameter.focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            );
            Line::from(vec![
                Span::styled(
                    format!("{:<label_width$}", parameter.label),
                    Style::default().fg(palette.structure_dim),
                ),
                value,
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.panel)),
        inner,
    );
}

fn render_scope(frame: &mut ratatui::Frame<'_>, area: Rect, view: &DemoView, palette: Palette) {
    let block = panel("SEQUENCE SCOPE", palette, false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width < 8 || inner.height < 5 {
        return;
    }
    for x in 0..inner.width {
        if x % (inner.width / 4).max(1) == 0 {
            for y in inner.y..inner.bottom() {
                frame.buffer_mut()[(inner.x + x, y)]
                    .set_char('┊')
                    .set_fg(palette.structure_dim)
                    .set_bg(palette.panel);
            }
        }
    }
    for row in 1..4 {
        let y = inner.y + row * inner.height / 4;
        for x in inner.x..inner.right() {
            frame.buffer_mut()[(x, y)]
                .set_char('·')
                .set_fg(palette.structure_dim)
                .set_bg(palette.panel);
        }
    }
    let pitch_height = inner.height.saturating_sub(4).max(1);
    let min_pitch = view.pitch_trace.iter().min().copied().unwrap_or(48);
    let max_pitch = view.pitch_trace.iter().max().copied().unwrap_or(72);
    let pitch_span = (max_pitch - min_pitch).max(1) as f64;
    for x in 0..inner.width {
        let source = x as usize * view.pitch_trace.len() / inner.width as usize;
        let pitch = view.pitch_trace[source.min(view.pitch_trace.len() - 1)];
        let scaled = (pitch - min_pitch) as f64 / pitch_span;
        let y = inner.y + (pitch_height - 1) - (scaled * (pitch_height - 1) as f64).round() as u16;
        frame.buffer_mut()[(inner.x + x, y)]
            .set_char('⠤')
            .set_fg(palette.primary)
            .set_bg(palette.panel);
    }
    let velocity_base = inner.bottom() - 1;
    for (index, velocity) in view.velocity_trace.iter().enumerate() {
        let x = inner.x + ((index * inner.width as usize) / view.velocity_trace.len()) as u16;
        let height = ((*velocity as u16 * 3) / 127).min(3);
        for rise in 0..height {
            frame.buffer_mut()[(x, velocity_base.saturating_sub(rise))]
                .set_char('│')
                .set_fg(palette.structure)
                .set_bg(palette.panel);
        }
    }
    let playhead_x = inner.x + ((view.playhead * inner.width as usize) / view.steps.len()) as u16;
    for y in inner.y..inner.bottom() {
        frame.buffer_mut()[(playhead_x.min(inner.right() - 1), y)]
            .set_char('│')
            .set_fg(palette.live)
            .set_bg(palette.panel);
    }
    let label = format!("PLAYHEAD {:02}", view.playhead + 1);
    frame.buffer_mut().set_string(
        (playhead_x + 1).min(inner.right().saturating_sub(label.len() as u16)),
        inner.y,
        label,
        Style::default().fg(palette.live).bg(palette.panel),
    );
}

fn render_modulation(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    view: &DemoView,
    palette: Palette,
) {
    let block = panel("MODULATION", palette, false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut lines = Vec::new();
    for route in &view.routes {
        let width = 8usize;
        let level = usize::from(route.level).min(width);
        let meter = format!("{}{}", "━".repeat(level), "·".repeat(width - level));
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<7} → {:<9} ", route.source, route.destination),
                Style::default().fg(palette.primary),
            ),
            Span::styled(
                format!("{:>6} ", route.amount),
                Style::default().fg(palette.value),
            ),
            Span::styled(
                meter,
                Style::default().fg(if route.active {
                    palette.live
                } else {
                    palette.structure_dim
                }),
            ),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.panel)),
        inner,
    );
}

fn render_context(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    view: &DemoView,
    palette: Palette,
    mode: LayoutMode,
) {
    let block = panel("CONTEXT / SC SOURCE", palette, false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if mode == LayoutMode::Standard {
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{:<24}", view.context_label),
                    Style::default().fg(palette.structure_dim),
                ),
                Span::styled(
                    "Ctrl-Enter evaluate   Esc restore last good   m route modulation",
                    Style::default().fg(palette.primary),
                ),
            ]),
            Line::from(Span::styled(
                view.source_lines.first().cloned().unwrap_or_default(),
                Style::default().fg(palette.value),
            )),
            Line::from(vec![
                Span::styled(
                    view.source_lines.get(1).cloned().unwrap_or_default(),
                    Style::default().fg(palette.value),
                ),
                Span::styled(
                    if view.staged { "  STAGED" } else { "" },
                    Style::default()
                        .fg(palette.edit)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().bg(palette.panel)),
            inner,
        );
    } else {
        let routes = view
            .routes
            .iter()
            .map(|route| format!("{}→{} {}", route.source, route.destination, route.amount))
            .collect::<Vec<_>>()
            .join("  ·  ");
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{}  ", view.context_label),
                    Style::default().fg(palette.structure_dim),
                ),
                Span::styled(routes, Style::default().fg(palette.value)),
                Span::styled("  STAGED", Style::default().fg(palette.edit)),
            ]))
            .style(Style::default().bg(palette.panel)),
            inner,
        );
    }
}

fn render_commands(frame: &mut ratatui::Frame<'_>, area: Rect, palette: Palette, mode: LayoutMode) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(palette.structure))
        .style(Style::default().bg(palette.panel));
    let line =
        "F1 PERFORM  F2 SEQUENCE  F3 DEVICES  F4 CODE  F5 ROUTES  F6 LOG  t THEME  ? HELP  q QUIT";
    let text = if mode == LayoutMode::Standard {
        vec![Line::from(vec![
            Span::styled(line, Style::default().fg(palette.value)),
            Span::styled("  CLOCK INTERNAL", Style::default().fg(palette.live)),
        ])]
    } else {
        vec![
            Line::from(Span::styled(
                "F1 PERFORM  F2 SEQUENCE  F3 DEVICES  F4 CODE  F5 ROUTES  F6 LOG",
                Style::default().fg(palette.value),
            )),
            Line::from(Span::styled(
                "t THEME  ? HELP  q QUIT  |  CLOCK INTERNAL",
                Style::default().fg(palette.live),
            )),
        ]
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().bg(palette.panel))
            .block(block),
        area,
    );
}

fn render_too_small(frame: &mut ratatui::Frame<'_>, area: Rect, palette: Palette) {
    frame.render_widget(
        Paragraph::new("INDEX // FIRST LIGHT\n\nTERMINAL TOO SMALL\nMINIMUM 80×30")
            .style(Style::default().fg(palette.value).bg(palette.background))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(palette.structure)),
            ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn snapshot(width: u16, height: u16, theme: Theme) -> (String, u64) {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let view = DemoView::canonical();
        terminal
            .draw(|frame| render(frame, frame.area(), &view, theme))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..height)
            .map(|y| {
                let mut line = (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>();
                while line.ends_with(' ') {
                    line.pop();
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut hash = 0xcbf29ce484222325u64;
        for cell in buffer.content() {
            let style = format!(
                "{}|{:?}|{:?}|{:?}",
                cell.symbol(),
                cell.fg,
                cell.bg,
                cell.modifier
            );
            for byte in style.bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        (text, hash)
    }

    #[test]
    fn standard_120x40_snapshot() {
        let (screen, hash) = snapshot(120, 40, Theme::AmberCga);
        assert!(screen.contains("INDEX // FIRST LIGHT"));
        assert!(screen.contains("Pbind(\\degree"));
        assert_eq!(hash, 6_152_696_802_077_356_758);
    }

    #[test]
    fn compact_80x30_snapshot() {
        let (screen, hash) = snapshot(80, 30, Theme::AmberCga);
        assert!(screen.contains("INDEX // FIRST LIGHT"));
        assert!(screen.contains("lfo_1→pitch"));
        assert_eq!(hash, 13_929_807_287_947_209_658);
    }

    #[test]
    fn blue_is_geometry_preserving_palette_substitution() {
        let (amber, _) = snapshot(120, 40, Theme::AmberCga);
        let (blue, _) = snapshot(120, 40, Theme::ConverterBlue);
        assert_eq!(amber, blue);
        assert_ne!(palette(Theme::AmberCga), palette(Theme::ConverterBlue));
    }

    #[test]
    fn exact_layout_regions() {
        let standard = regions(Rect::new(0, 0, 120, 40));
        assert_eq!(standard.pattern, Rect::new(0, 2, 84, 10));
        assert_eq!(standard.parameters, Rect::new(84, 2, 36, 19));
        assert_eq!(standard.scope, Rect::new(0, 12, 84, 19));
        assert_eq!(standard.context, Rect::new(0, 31, 120, 6));
        let compact = regions(Rect::new(0, 0, 80, 30));
        assert_eq!(compact.scope, Rect::new(0, 12, 51, 12));
        assert_eq!(compact.parameters, Rect::new(51, 12, 29, 12));
    }
}
