# Index

Index is a native, cross-platform sequencing instrument: a Rust/Ratatui frontend supervises a headless SuperCollider `sclang` process. It sequences only—there is intentionally no synthesis, `scsynth`, audio processing, ADAT, mixer, JACK, or PipeWire functionality.

Rust and SuperCollider remain separate because Rust is a good home for native UI, project persistence, MIDI portability, and process supervision, while SuperCollider already provides the pattern/stream/event model through `Pdef`, `Pbindef`, `Pdefn`, Patterns, Streams, Events, and Clocks. Normal edits use structured OSC; `/code/eval` is an explicit advanced escape hatch.

## Install and run

On macOS, install Rust and SuperCollider. With Homebrew: `brew install rust` and `brew install --cask supercollider`. Then:

```sh
cargo run -- doctor
cargo run -- run examples/first-light
./scripts/smoke-test
cargo test
```

Set `INDEX_SCLANG=/absolute/path/to/sclang` to override discovery. The built-in Monitor destination makes the demo usable without MIDI hardware. A future native-port selector persists logical destination names alongside the system port name; absent ports are diagnosed rather than silently remapped.

## Controls

`F1` Perform, `F2` Sequence, `F3` Devices, `F4` Code/Log; arrows or `hjkl` move; `Tab` changes panes; `Enter` edits the selected degree; `Space` plays/stops; `?` shows contextual help; `q` quits and stops the supervised engine. The tracker currently edits degree values in the UI; duration, velocity, channel, and destination are displayed and supported by the structured protocol/project format.

## Project layout and protocol

Projects contain `project.toml`, `patterns/bass.toml`, and free `live/session.scd`. Writes use a temporary file and rename. The protocol is documented in `ARCHITECTURE.md`; the core paths are `/index/v1/hello`, `/ready`, `/transport/play`, `/transport/stop`, `/tempo`, `/pattern/set`, `/pattern/commit`, `/code/eval`, `/event/midi`, `/error`, and `/state`.

## Limitations and direction

This is the first vertical slice: one managed pattern, one monitor destination, degree editing, and a compact UI. Physical MIDI output discovery and richer cell editors are intentionally next steps. The longer-term direction includes modular sequencing tools inspired by ixiQuarks/ixi lang, reusable agents, arrangements/scenes, device profiles, modulation, and NerdSEQ/Dyaxis/OSC-node integrations. Those are future work, not hidden promises of this prototype.
