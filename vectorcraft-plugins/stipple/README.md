# Stipple

A VectorCraft (the ArtCraft/"VectorCraft" Illustrator-class open-source app,
https://github.com/storytold/vectorcraft) object-filter plug-in, built against its documented
WASM plug-in ABI (`docs/plugins.md` in that repo).

Fills the selected path(s) with a jittered grid of small dots clipped to the shape's own fill
area (honours nonzero/evenodd), coloured with the object's own solid fill colour. This is the
rough equivalent of Astute Graphics' "Stipplism" plugin for Adobe Illustrator — VectorCraft has
no built-in stipple/dot-texture tool (it does have Round Corners and Scribble natively, which is
why this targets a genuine gap instead of duplicating something already there).

## Build

```sh
rustup target add wasm32-unknown-unknown
./build.sh
```

Produces `target/wasm32-unknown-unknown/release/vectorcraft_plugin_stipple.wasm`.

## Install

- In the app: **File > Open** that `.wasm` file, or **Object > Plug-ins > Install Plug-in…**.
- Over the control port: `{"id":1,"method":"engine.execute","params":{"command":"plugin.install","params":{"path":"/absolute/path/to/vectorcraft_plugin_stipple.wasm"}}}`.
- Or drop it in the *Additional Plug-ins Folder* (Edit > Preferences > Performance & Storage) to
  have it load on every start.

## Use

Select a filled path or compound path, then **Object > Plug-ins > Stipple**. Parameters:

- **spacing** (1–200 pt): grid cell size.
- **dotSize** (5–100%): dot diameter as a percentage of spacing.
- **jitter** (0–100%): how much each dot wanders off the grid (0 = perfectly regular, 100 = fully
  scattered within its cell). Jitter is seeded from the object's id, so re-running on the same
  object gives the same result.

Verified against `vectorcraft-plugins`' own `wasmi` host with a temporary smoke test (a 100×100 pt
square at 10 pt spacing produces a ~10×10 grid of dots, keeps the original fill colour, and an
empty selection reports a clean error) — not committed here since it needs the upstream crate as a
dev-dependency; re-create it from `crates/plugins/tests/abi.rs` in the vectorcraft repo if you want
to re-check after changes.

## What this isn't

The v1 plug-in ABI only supports object filters (one-shot, selection → new geometry) and live
effects that rewrite geometry — no custom tools, no paint changes, no panels. So this acts on a
selection once rather than being a live, editable "Stipple" appearance effect the way Illustrator
plugins often are.
