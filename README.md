# Index instrument UI

This milestone is a UI-only Ratatui visual demo. It presents a dense 16-step command-station layout with an Amber CGA default theme, Converter Blue alternate theme, parameter panel, animated sequence scope, modulation summary, pop-down editor, help overlay, and local Monitor activity. It does not launch SuperCollider, access MIDI, send OSC, or mutate project files.

Run:

\`\`\`sh
cargo run -- ui-demo
\`\`\`

Controls:

- Left/Right or \`h/l\`: move edit cursor
- Up/Down or \`j/k\`: change selected note
- \`J/K\` or Shift+Up/Down: octave transpose
- \`[\` / \`]\`: change velocity
- \`-\` / \`+\`: change duration
- \`r\`: toggle rest
- Space: play/stop animation
- \`t\`: switch Amber CGA / Converter Blue
- Enter: open the parameter pop-down editor
- Escape: close an overlay
- \`?\`: toggle help
- \`q\`: quit

The reusable widgets live in \`crates/instrument-ui\` and accept plain view-state structures. They have no dependency on sequencer, mixer, SuperCollider, MIDI, OSC, or persistence business logic.

Validate with:

\`\`\`sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
\`\`\`
