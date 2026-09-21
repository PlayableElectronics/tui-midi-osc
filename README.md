# Index instrument UI

This milestone is a static, deterministic Ratatui implementation of the canonical INDEX command-station design in `design/`. It presents the fixed 16-step pattern, selected-step inspector, pitch/velocity scope, modulation routes, contextual SuperCollider fragment, and stable command strip in Amber CGA or Converter Blue. It does not launch SuperCollider, access MIDI, send OSC, animate, or mutate project files.

Run:

```sh
cargo run -- ui-demo
```

The static review demo accepts `t` to verify the geometry-preserving blue palette substitution and `q` to quit.

The reusable widgets live in `crates/instrument-ui` and accept plain view-state structures. They have no dependency on sequencer, mixer, SuperCollider, MIDI, OSC, or persistence business logic.

Validate with:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```
