# Simorgh brand assets

Primary mark: **Waveflight** — a waveform that lifts into three tail feathers.
Speech becoming flight; the mythical bird who understood every tongue.

## Files

### `svg/` (source of truth — always prefer these)

| File | Use |
| --- | --- |
| `simorgh-mark.svg` | Primary mark, gradient for **light** backgrounds (`#2783DE` → `#46A171`) |
| `simorgh-mark-dark.svg` | Primary mark, gradient for **dark** backgrounds (`#5E9FE8` → `#72BC8F`) |
| `simorgh-mark-mono.svg` | Single-color glyph — replace the stroke color or use `currentColor` |
| `simorgh-icon.svg` | App icon: mark in a dark rounded tile (use at 64px and larger) |
| `simorgh-icon-small.svg` | Simplified icon for 16–48px: wing only, thicker strokes |
| `simorgh-lockup-en.svg` | Horizontal lockup, English wordmark + tagline |
| `simorgh-lockup-fa.svg` | Horizontal lockup, Persian (RTL: mark on the right) |

### `png/` (rendered exports)

- `icon-16/32/48.png` — from the **simplified** small-size variant
- `icon-64/128/256/512/1024.png` — from the full app icon
- `simorgh-mark-512.png`, `simorgh-mark-dark-512.png`, `simorgh-mark-mono-512.png` — transparent backgrounds
- `simorgh-lockup-en.png`, `simorgh-lockup-fa.png`

## Usage rules

- Use the **full mark** at 64px and above; switch to the **simplified wing icon** below 48px (favicons, tray icons).
- Light-background gradient on light surfaces, dark-background gradient on dark surfaces — do not swap them.
- The mono glyph is for single-color contexts (terminal splash, engraving, stickers, muted UI states).
- Persian lockup is RTL: the mark sits on the right. Do not mirror the mark itself.
- Wordmark fonts: bold geometric sans for “Simorgh” (Arial/Inter-class); `Noto Sans Arabic` or `Vazirmatn` for “سیمرغ”.
- Keep clear space around the mark of at least the height of one tail feather stroke.

## Colors

| Token | Light bg | Dark bg |
| --- | --- | --- |
| Gradient start (blue) | `#2783DE` | `#5E9FE8` |
| Gradient end (green) | `#46A171` | `#72BC8F` |
| Mono | `#2C2C2B` | `#FFFFFF` |
| Icon tile | — | `#191919` |
