# Hue/Saturation/Brightness (Color Adjust)

A VectorCraft object-filter plug-in. Shifts hue, scales saturation and shifts brightness for every
colour of the selection — solid fills and strokes, gradient stops, freeform gradient points — each
kept in its own colour model, the same recursive walk the built-in Desaturate example uses.

The rough equivalent of Astute Graphics' "Phantasm" HSB adjustments. VectorCraft's own Edit > Edit
Colors only exposes per-channel balance (R/G/B or C/M/Y/K add/subtract), not hue/saturation/
brightness.

## Build

```sh
rustup target add wasm32-unknown-unknown
./build.sh
```

## Install

Same as the other plugins here: **File > Open** the `.wasm`, or `plugin.install` on the control
port, or drop it in the Additional Plug-ins Folder.

## Use

Select some paths, then **Object > Plug-ins > Hue/Saturation/Brightness**. Params:

- **hue** (-180 to 180°): hue rotation.
- **saturation** (-100 to 100%): scales saturation; -100 fully desaturates.
- **brightness** (-100 to 100%): shifts lightness (RGB/gray) or L (Lab).

RGB and gray colours are adjusted exactly via HSL. CMYK converts its C/M/Y tint to RGB at the
current K, adjusts, and solves back at the same K — so pure black (K=1) is left alone, same as a
human would expect. Lab adjusts L directly and rotates/scales the a/b chroma — these two are
approximations, not a claim of matching another application pixel-for-pixel (the same caveat
VectorCraft's own docs make about its native tools).

Verified with unit tests (`cargo test`: HSL round-trip, hue rotation, clamping, CMYK black
invariance, Lab hue-rotate reversibility) and against `vectorcraft-plugins`' own `wasmi` host the
same way as the other plugins here.
