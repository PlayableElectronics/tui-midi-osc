# State semantics

| State | Foreground | Background | Border | Rule |
|---|---|---|---|---|
| normal | primary/value | panel | structure_dim | default |
| focused panel | primary/value | panel | structure | one panel only |
| selected step | value | panel | structure | persistent selection |
| edit cursor | value | edit | structure | one field/cell maximum |
| playhead | background | live | live | moves with clock |
| selected + playhead | edit | live | structure | both remain distinguishable |
| staged code | value | panel | edit | not yet evaluated |
| active route pulse | live | panel | structure_dim | brief foreground/meter pulse |
| warning | primary | panel | edit | recoverable, no red |
| error | value | error | error | fault only |

Focus, selection, and playhead are independent states. They must never be represented by the same colour.

## Interaction model

- Arrow keys move focus within the current panel.
- `Tab` / `Shift-Tab` move between panels in a fixed loop.
- `Enter` enters or commits editing.
- `Esc` cancels editing or closes a modal.
- Space toggles transport only from perform/sequence views.
- `e` opens the focused object's SuperCollider source in the context panel.
- `Ctrl-Enter` evaluates the staged fragment; the previous working fragment remains recoverable.
- `m` opens the route editor for the focused parameter.
- Function keys change views without changing transport state.

Code editing is contextual: an object exposes its code and parameters without requiring the user to leave the instrument. New code may declare parameter metadata so the TUI can render controls automatically; unknown metadata remains editable as text.
