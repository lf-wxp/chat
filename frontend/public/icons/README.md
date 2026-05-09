# PWA Icons

This folder holds the Progressive Web App icons referenced by `manifest.json`.

## What ships today

- `icon.svg` — Source vector icon (512×512 viewBox). Used as the
  fallback and the authoritative artwork. `manifest.json` lists PNG
  variants that are expected to be generated from this SVG at build
  time.

## Regenerating the PNG variants

Browsers that honour the `manifest.json` need raster PNGs in the
following sizes: **72, 96, 128, 144, 152, 192, 384, 512**. Generate
them from `icon.svg` using any rasteriser you prefer; two common
options are below.

### Using ImageMagick

```bash
cd frontend/public/icons
for size in 72 96 128 144 152 192 384 512; do
  magick -background none -density 400 icon.svg \
    -resize ${size}x${size} icon-${size}x${size}.png
done
```

### Using rsvg-convert (librsvg)

```bash
cd frontend/public/icons
for size in 72 96 128 144 152 192 384 512; do
  rsvg-convert -w ${size} -h ${size} icon.svg -o icon-${size}x${size}.png
done
```

## Why PNGs are not committed

Binary PNG assets churn the repository noisily and double-encode the
SVG's geometry. Keep the SVG source under version control and
regenerate PNGs in CI / at packaging time. The `Dockerfile` build
stage is a good home for the regeneration step.

## Maskable variants

The 192×192 and 512×512 entries in `manifest.json` declare
`purpose: "any maskable"`. Make sure the produced PNGs honour the
safe zone (80% centred) so Android launchers can crop the icon
without clipping the chat bubbles. The source SVG is already
designed with that safe zone in mind.
