# Palettes

All values are sRGB hex and must be represented with `ratatui::style::Color::Rgb`.

## Amber — canonical

| Token | Hex | Use |
|---|---:|---|
| `background` | `#120900` | terminal canvas |
| `panel` | `#080300` | panel interiors |
| `structure_dim` | `#8A5700` | grids, inactive borders, muted labels |
| `structure` | `#D78A00` | focused borders, headings |
| `primary` | `#FFB000` | normal text and traces |
| `value` | `#FFE1A0` | pitches, numbers, selected values |
| `edit` | `#E84A1A` | edit cursor and changed field |
| `live` | `#63D86B` | playhead and activity only |
| `error` | `#FF3B30` | failure only |

## Converter Blue — alternate

| Token | Hex |
|---|---:|
| `background` | `#02072A` |
| `panel` | `#000317` |
| `structure_dim` | `#165D82` |
| `structure` | `#00B8D9` |
| `primary` | `#00D9FF` |
| `value` | `#E8FBFF` |
| `edit` | `#FF315F` |
| `live` | `#36FF77` |
| `error` | `#FF315F` |

The blue theme changes colour tokens only. Geometry, weight, state semantics, and content remain identical.

## Terminal fallback

TrueColor is the intended presentation. If the terminal lacks TrueColor, quantise these RGB values to xterm-256 at runtime. Do not replace them with `Red`, `Green`, `Blue`, `Yellow`, or other named colours.
