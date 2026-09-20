# Index architecture

The first slice is deliberately two-process and sequencing-only. Rust owns project files, the Ratatui UI, supervision, OSC transport, and the native MIDI boundary. A headless `sclang` owns a real managed `Pbindef(\bass, ...)`, its clock, and normalized event production. Quitting the UI stops the supervised child so the prototype cannot leave an unintended `sclang` behind.

The OSC protocol is versioned under `/index/v1`. Rust sends `/hello`, transport commands, `/tempo`, structured `/pattern/set` followed by `/pattern/commit`, and explicit `/code/eval`. SC replies `/ready`, `/state`, `/event/midi`, and `/error`. Pattern arrays are comma-separated strings in v1 to keep the schema stable across SC and Rust without generating source code.

Future work, not implemented in this slice: reusable sequencing tools/agents; tracker and indexed views over managed SC objects; arrangements and scenes; modulation matrices for event/control messages; device profiles; native SC livecoding UX; and NerdSEQ, Dyaxis, and custom OSC-node integration. Clock sources should later implement `Internal`, `MIDI input`, `NerdSEQ`, and `OSC` behind a common interface.
