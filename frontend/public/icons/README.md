# PWA Icons

This folder holds the Progressive Web App icons referenced by
`manifest.json` (one level up).

## Source of truth

- `icon.svg` — the authoritative artwork (512×512 viewBox, maskable
  safe-zone designed-in). Kept under version control.
- `icon-{72,96,128,144,152,192,384,512}x{size}.png` — rasterised at
  build time from `icon.svg`. These files **do not live in git**;
  they are produced by either the Dockerfile or the local helper
  script below.

## How the PNGs are produced

### In CI / production

The Dockerfile's `frontend-builder` stage installs `librsvg2-bin` and
rasterises all eight sizes before `trunk build`. Trunk's
`copy-dir public` directive then picks them up into `dist/icons/`.
No manual step is required to ship to production.

### Locally (for dev preview)

```bash
cargo make pwa-icons       # Preferred
# or directly:
./scripts/generate-pwa-icons.sh
```

Both invocations require `rsvg-convert` from librsvg:

```bash
# macOS
brew install librsvg

# Debian / Ubuntu
apt-get install librsvg2-bin

# Fedora / RHEL
dnf install librsvg2-tools
```

## Why PNGs are not committed

1. Binary churn — regenerating them shouldn't pollute git diffs.
2. Reproducibility — the SVG is canonical; the PNGs are a derived
   artefact, like a build output.
3. Docker layer efficiency — regenerating PNGs inside the build
   stage keeps them in a cache layer that invalidates only when
   `icon.svg` changes.

## Maskable safe zone

`manifest.json` marks the 192/512 entries as
`"purpose": "any maskable"`. `icon.svg` already reserves an 80%
centred safe zone so Android launchers can apply their circular /
rounded-square masks without clipping the chat bubbles.

If you edit `icon.svg`, keep all meaningful pixels inside a
circle of radius ≈205px centred at (256, 256) to preserve the safe
zone.
