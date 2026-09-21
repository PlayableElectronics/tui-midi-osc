# Instrument UI architecture

`crates/instrument-ui` is a reusable Ratatui widget crate. It contains the instrument shell, semantic themes, square panels, responsive 16-step grid, parameter list, plotting scope, meter, sparklines, modulation summary, pop-down editor, and help overlay.

Widgets consume plain view-state structures. They do not know about sequencing engines, SuperCollider, MIDI, OSC, mixers, devices, persistence, or process supervision. The root `ui-demo` command adapts local sample state into those structures and animates it with a local monotonic clock.

The active layout is intentionally a professional terminal instrument: Amber CGA is the default palette, Converter Blue is an alternate, edit focus is red-orange, and live/playhead state is green. The grid renders as 16 columns when space permits and 8×2 on narrower terminals. A minimum-size fallback prevents out-of-bounds rendering.

This milestone does not alter backend integration. The existing backend experiments remain outside the active UI demo path.
