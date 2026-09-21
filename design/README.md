# INDEX visual contract

This directory is the canonical visual reference for INDEX and for future instruments built with `instrument-ui`.

The first implementation target is a **static, deterministic screen**. It must match the geometry, hierarchy, and semantic colour use here before animation, engine integration, or additional widgets are accepted.

## Direction

INDEX should feel like a dedicated late-1980s/early-1990s music workstation: dense, legible, fast, and calm. It is not a dashboard and not a collection of colourful boxes.

References:

- Urr Converter: https://www.urr.ca/software/converter/screenshots.htm
- Ratatui scope example: https://github.com/ratatui/ratatui/tree/main/examples/apps/scope
- Dronage Terminal and Grampus for terminal craft and density
- ER-101, Notator, ORCA, Blue, and ixi quarks for instrument behaviour

## Files

- [visual-contract.md](visual-contract.md) — non-negotiable implementation and review rules
- [palette.md](palette.md) — exact semantic colours
- [layout.md](layout.md) — fixed 120×40 and compact 80×30 geometry
- [state-semantics.md](state-semantics.md) — focus, edit, playhead, warning, and staged states
- [mockups/amber-command-station.svg](mockups/amber-command-station.svg) — canonical visual target

## Review gate

A UI change is not ready until it includes deterministic Ratatui `TestBackend` snapshots at 120×40 and 80×30 and a real-terminal screenshot. Review the screenshot before wiring it to SuperCollider.

The mixer may reuse the visual system later, but INDEX remains a sequencer. No mixer channels or ADAT controls belong in this screen.
