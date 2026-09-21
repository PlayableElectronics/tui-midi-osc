# Visual contract

## Non-negotiable rules

1. Use explicit `Color::Rgb` values from [palette.md](palette.md). Named ANSI colours are forbidden in production themes because terminal mappings vary wildly.
2. At least 80% of the screen remains `background` or `panel`. Accent colour is ink, not wallpaper.
3. Never fill a panel with red, green, blue, or amber. Filled accent backgrounds are limited to one cursor cell, one playhead cell, or a modal title.
4. Borders are one-cell square box drawing. No rounded borders, drop shadows, gradients, or ornamental double framing.
5. Titles are left aligned and embedded in the top border: `┤ PATTERN ├`.
6. Values are brighter than labels. Structure is dimmer than content.
7. Green means live time only: transport, playhead, MIDI/OSC activity, and active route pulses.
8. Orange-red means the edit cursor only. Red means an actual fault.
9. The sequence scope is a thin Braille/line trace over a sparse grid. It is never a filled histogram and never decorative noise.
10. The bottom command strip is persistent and stable across views.
11. At 120×40, the user sees pattern, parameters, scope, modulation routes, transport, and commands simultaneously.
12. At 80×30, information may collapse but commands and current context never disappear.

## Typography

Use the terminal's monospace font. Uppercase is reserved for panel titles, transport state, and function-key labels. Values and code preserve case. Do not simulate scanlines, phosphor bloom, or CRT distortion in the TUI.

## Density

This is an instrument, not a settings page. Empty space must carry hierarchy. A large blank inspector while the scope is overloaded is a layout failure.

## Implementation order

1. Hard-coded demo model
2. Deterministic 120×40 snapshot
3. Deterministic 80×30 snapshot
4. Real-terminal screenshot in amber
5. Keyboard focus/edit behaviour
6. Live model binding
7. Animation and activity pulses

Do not change transport, OSC, MIDI, or SuperCollider code during steps 1–4.
