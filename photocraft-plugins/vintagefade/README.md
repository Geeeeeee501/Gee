# Vintage Fade

A PhotoCraft (the ArtCraft/"PhotoCraft" Photoshop-class open-source app,
https://github.com/storytold/photocraft) filter plug-in, built against its documented WASM
plug-in ABI (`docs/plugins.md` in that repo).

A one-click "vintage/film" look: faded contrast, a warm-highlight/cool-shadow split tone, grain
and a corner vignette, all in one filter. PhotoCraft already has every individual Photoshop
adjustment this is built from (Curves, Color Balance, Add Noise, Lens Correction's vignette) — its
own self-graded parity doc rates "every Filter and Image Adjustments item" done. This plug-in's
value is the packaged, tasteful-by-default combination in one click instead of stacking four
adjustment layers by hand, the same way most real third-party Photoshop filter packs (Nik Color
Efex, Alien Skin Exposure) are curated recipes, not new pixel math.

I looked for a genuinely *missing* pixel operation first (channel-shift/chromatic-aberration-style
glitch effects, duotone/split-toning, halftone screening) but PhotoCraft's native filter and
adjustment catalog already covers essentially all of that — see its own `docs/target-app-parity.md`
("Filters (incl. Filter Gallery ×47, Liquify, blurs, render) ~88%"). A packaged recipe was the
honest, non-redundant thing left to build.

## Build

```sh
rustup target add wasm32-unknown-unknown
./build.sh
```

## Install

- In the app: **Filter > Plug-ins > Install Plug-in…**, pick the `.wasm` file.
- Over the control port: `plugin.install {"path": "/absolute/path/to/photocraft_plugin_vintagefade.wasm"}`.
- Or drop it in the Additional Plug-ins Folder (Edit > Preferences > Plug-ins) to load on every
  start.

## Use

**Filter > Plug-ins > Vintage Fade** on the active pixel layer. Params:

- **fade** (0–100%): lifts blacks and pulls down whites towards mid-grey (classic faded-film low
  contrast).
- **warmth** (-100 to 100%): split tone — positive warms highlights and cools shadows; negative
  reverses it.
- **vignette** (0–100%): corner darkening, relative to the whole canvas (not just the layer's
  content), so it stays correct even on a layer smaller than the canvas.
- **grain** (0–100%): per-pixel grain, seeded from each pixel's absolute position so it's
  identical regardless of how the host splits the image into bands — no visible seams.

Transparent pixels are left untouched, same convention as the built-in Invert example plug-in.
RGB images get the full recipe (including the warm/cool split tone); grayscale, CMYK and other
modes get fade/grain/vignette only, since the split tone needs distinct R/G/B channels.

Verified with unit tests (`cargo test`: vignette falloff, fade math, split-tone direction, grain
determinism — one test caught a sign error in the split-tone formula that would have cooled
highlights and warmed shadows backwards) and against `photocraft-plugins`' own `wasmi` host via
its `Plugin::apply` API (vignette darkens corners vs. centre, warmth shifts a highlight's red up
and blue down, a fully transparent pixel is left alone).
