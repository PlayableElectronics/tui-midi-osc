# Index

Index is a native, cross-platform sequencing instrument: a Rust/Ratatui frontend supervises a headless SuperCollider `sclang` process. It sequences only—there is intentionally no synthesis, `scsynth`, audio processing, ADAT, mixer, JACK, or PipeWire functionality.

Rust and SuperCollider remain separate because Rust is a good home for native UI, project persistence, MIDI portability, and process supervision, while SuperCollider already provides the pattern/stream/event model through `Pdef`, `Pbindef`, `Pdefn`, Patterns, Streams, Events, and Clocks. Normal edits use structured OSC; `/code/eval` is an explicit advanced escape hatch.

## Install and run

On macOS, install Rust and SuperCollider. With Homebrew: `brew install rust` and `brew install --cask supercollider`. Then:

```sh
cargo run -- doctor
cargo run -- run examples/first-light
# deterministic Monitor-mode demo when sclang is unavailable
cargo run -- demo examples/first-light
./scripts/smoke-test
cargo test
```

Set `INDEX_SCLANG=/absolute/path/to/sclang` to override discovery. The built-in Monitor destination makes the demo usable without MIDI hardware. The Devices screen lists native ports and Enter opens a selected port; a missing saved destination is never silently remapped.

## Controls

`F1` Perform, `F2` Sequence, `F3` Devices, `F4` Log; `h/l` select a tracker cell, `j/k` select a row, `Tab` changes screens, `Enter` edits degree/duration/velocity/channel/destination, `i` commits immediately, `b` commits on the next bar, `Space` plays/stops, `:` enters `:play`, `:stop`, `:commit`, or `:bar`, `?` shows help, and `q` quits with terminal restoration. `cargo run -- demo examples/first-light` provides the same visible sequence and simulated playhead through the Monitor backend without launching SC.

## Project layout and protocol

Projects contain metadata/reference and the selected output device in `project.toml`, the authoritative managed pattern in `patterns/bass.toml`, and free `live/session.scd`. Every valuable file is written through a same-directory temporary file, synced, and renamed. The protocol is documented in `ARCHITECTURE.md`; `./scripts/smoke-test` uses a temporary project copy.

## Limitations and direction

The managed pattern is authoritative in `patterns/bass.toml`; `project.toml` contains metadata, the pattern reference, and one exact-name output configuration. In the real path SC owns the musical clock, Pbind stream, quantized replacement and production MIDI; Rust displays SC telemetry and supervises the child. The Monitor/demo path is intentionally simulated for terminal demonstrations and tests. Startup retries `/hello`, synchronizes the saved project, and only then enables transport/editing. SC evaluation and validation errors are recoverable and appear in the Log screen, while child-process failure is fatal.
