# Instrument UI architecture

`crates/instrument-ui` is a reusable Ratatui widget crate. It contains the canonical instrument shell, semantic RGB themes, square panels, 8×2 step grid, selected-step parameters, pitch/velocity scope, modulation routes, contextual source panel, and persistent command strip.

Widgets consume plain view-state structures. They do not know about sequencing engines, SuperCollider, MIDI, OSC, mixers, devices, persistence, or process supervision. The root `ui-demo` command adapts one deterministic hard-coded model into those structures. It performs no animation or backend work.

The active layout follows the exact coordinates in `design/layout.md`: 120×40 standard and 80×30 compact. Amber CGA is the default palette, Converter Blue is a colour-only substitution, edit focus is orange-red, and live/playhead state is green. A minimum-size fallback prevents out-of-bounds rendering.

This milestone does not alter backend integration. Existing backend experiments remain outside the active UI demo path.
