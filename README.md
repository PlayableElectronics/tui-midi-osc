# Index demo sequencer

This branch contains a focused, sequencing-only Ratatui demo core: a visible 16-step terminal instrument with a local Monitor playback simulation. It is intentionally self-contained. SuperCollider, MIDI, OSC, devices, live coding, revisions, transactions and dynamic UI are not part of this demo build.

Run it with:

```sh
cargo run -- demo examples/first-light
```

The demo loads `project.toml` and the authoritative `patterns/bass.toml`, displays sixteen steps horizontally, and saves edits back to those files. Playback uses each step's duration and the current BPM; rests produce no Monitor event.

Controls:

| Key | Action |
| --- | --- |
| Left/Right or h/l | Select previous/next step |
| Up/Down or j/k | Transpose selected note by one semitone |
| Shift+Up/Down or J/K | Transpose by one octave |
| `[` / `]` | Decrease/increase velocity |
| `-` / `+` | Decrease/increase duration |
| r | Toggle note/rest |
| Space | Play/stop |
| `,` / `.` | Decrease/increase tempo |
| s | Save |
| ? | Show help in the status line |
| q | Quit |

The yellow cell is the edit cursor; the green cell is the moving playback cursor. The activity panel shows Monitor note events. The data model uses MIDI note numbers, velocities 0–127, four musical duration choices (1/8, 1/4, 1/2 and 1 beat), and tempos from 30–300 BPM.

Validation:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```
