# Demo core architecture

The active build is deliberately small. `src/main.rs` contains the Ratatui view, keyboard interaction, demo clock and Monitor activity log. `src/project.rs` contains the 16-step data model, validation, note-name conversion, and atomic TOML persistence.

The edit cursor and playback cursor are separate values. Playback advances on a monotonic deadline calculated as:

```text
step_seconds = 60 / BPM * step_duration_beats
```

Starting playback resets the playback cursor to step 1 and emits the first non-rest step immediately. Stopping clears the playback cursor and freezes the activity log. Tempo changes affect the next scheduled step. No production MIDI scheduling occurs in this build.

Project metadata lives in `project.toml`; the managed sixteen-step sequence lives in the referenced `patterns/bass.toml`. Saves write same-directory temporary files, sync them, and rename them into place.

The earlier supervised SuperCollider, OSC and native MIDI experiments remain in repository history and inactive source files for reference. They are intentionally not compiled by this demo nucleus. Future integration must preserve this interaction model rather than reintroduce distributed editing state into the UI.
