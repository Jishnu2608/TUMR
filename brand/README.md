# Brand: the Toolchain Universal Mobile Runtime mark

The product mark (owner request, 2026-11-14): a phone whose screen shows a coding agent's prompt
`›_` — a mobile runtime for AI coding agents. Studio / Signal, the website's palette (owner
decision D1, 2026-11-25; the silhouette is unchanged from the Graphite & Lime original).

| File | Use |
|---|---|
| `logo-mark.svg` | Master, 32 px and larger: website header and footer (inlined in `webapp/frontend/components/site/Wordmark.tsx`), desktop wizard header, large icon sizes |
| `logo-mark-small.svg` | Bold variant for 16–24 px: website favicon (`webapp/frontend/app/icon.svg`), small Windows icon sizes |
| `logo-1024.png` | 1024 px render of the master, for anything that needs a bitmap: agent plugin icons (`plugins/*/assets/icon.png`), the `.mcpb` bundle icon, the VS Code extension icon (256 px) |
| `logo-128.png` | 128 px render of the master, for store profiles (VS Code Marketplace publisher, Open VSX namespace) |

Colours: tile Ink/Graphite gradient `#2C333B` → `#1B1F24` (small variant: flat `#20252B`), phone
`#F2F0EA` (Porcelain) on `#20252B` (Ink), prompt Signal Blue `#6A86F2` (the light tint of
`#4263EB`, which would be too faint on the dark tile at 16 px). Keep the mark on its own tile; do
not recolour the prompt.

The name next to it is still the working name (open decision W1).

## Windows icon
`apps/desktop/icons/icon.ico` holds PNG images at 16, 20, 24, 32 (small variant) and 40, 48, 64,
128, 256 px (master). To rebuild it after changing the artwork: render each SVG at those sizes
with a transparent background (any headless browser), then pack the PNGs into one ICO
(little-endian `ICONDIR` + one `ICONDIRENTRY` per size, PNG data appended). Windows Vista and
later read PNG entries.
