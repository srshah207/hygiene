# Brand assets — sources & regeneration

## Source of truth

| File | Purpose |
| --- | --- |
| `../icon.svg` | Primary mark: gradient byte-ring dissolving into the swept gap + sparkle (64px and up). |
| `../icon-small.svg` | Small-size variant for 16/32/48px: flat single teal, no fade, chunkier sparkle. |
| `../logo.svg` | Horizontal lockup (badge + lowercase wordmark). Wordmark adapts to dark mode via `prefers-color-scheme`. |
| `social.html` | GitHub social preview source (1280x640). |
| `icon.html`, `icon-small.html`, `logo.html` | Viewport-filling wrappers used for rasterization. |

## Rendered artifacts (committed)

- `icon-16..512.png` — 16/32/48 from the small variant; 64/128/256/512 from the primary.
- `logo-800.png` — lockup raster (light ink, transparent background).
- `social-preview.png` — 1280x640 social card.
- `hygiene.ico` — PNG-embedded entries: 16/32/48 small variant, 64/128/256 primary.

## How to regenerate

Use any Chromium-based headless browser; one size per invocation:

```bash
msedge --headless=new --window-size=512,512 --default-background-color=00000000 \
       --screenshot=icon-512.png icon.html
```

Notes:

- `--default-background-color=00000000` keeps transparency.
- Assets containing text (`logo.html`) need `--blink-settings=preferredColorScheme=1`
  so the wordmark renders in its dark-ink light-scheme color.
- `hygiene.ico` is assembled from the PNGs with PNG-embedded entries (no legacy BMP).
