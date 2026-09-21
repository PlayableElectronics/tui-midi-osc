# Layout

Coordinates are zero-based terminal cells. Outer bounds include borders.

## Standard 120×40

| Region | x | y | w | h | Contents |
|---|---:|---:|---:|---:|---|
| status | 0 | 0 | 120 | 2 | name, project, transport, tempo, clock, I/O activity |
| pattern | 0 | 2 | 84 | 10 | 16 steps, two rows of eight |
| parameters | 84 | 2 | 36 | 19 | selected step and conditional data |
| scope | 0 | 12 | 84 | 19 | pitch trace, velocity trace, playhead |
| modulation | 84 | 21 | 36 | 10 | source → destination, amount, miniature meter |
| context | 0 | 31 | 120 | 6 | live-code fragment or focused object details |
| commands | 0 | 37 | 120 | 3 | stable function keys and one-line help |

The vertical split is fixed at column 84. The pattern cells are 10 columns wide with one-cell gaps. Each cell shows step number, pitch, velocity bar, and flags. The scope shares the same horizontal time direction as the pattern.

## Compact 80×30

| Region | x | y | w | h |
|---|---:|---:|---:|---:|
| status | 0 | 0 | 80 | 2 |
| pattern | 0 | 2 | 80 | 10 |
| scope | 0 | 12 | 51 | 12 |
| parameters | 51 | 12 | 29 | 12 |
| context | 0 | 24 | 80 | 3 |
| commands | 0 | 27 | 80 | 3 |

In compact mode, modulation routes become a single summary line in context. The selected parameter remains visible. No horizontal scrolling is required.

## Stable command strip

`F1 PERFORM  F2 SEQUENCE  F3 DEVICES  F4 CODE  F5 ROUTES  F6 LOG  t THEME  ? HELP  q QUIT`

The focused page may add shortcuts on the final line, but must not reorder these commands.
