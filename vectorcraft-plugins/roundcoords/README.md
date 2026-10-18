# Round Coordinates

A VectorCraft object-filter plug-in. Snaps every anchor (and, optionally, its handles) of the
selected paths/compound paths to the nearest multiple of a grid size.

VectorCraft has interactive Snap to Grid/Pixel for dragging, but no batch command to clean up
coordinates already on the page. Useful before sending art to a cutter, plotter or engraver, where
stray sub-unit jitter left by an import or a trace causes tiny, unwanted segments.

## Build

```sh
rustup target add wasm32-unknown-unknown
./build.sh
```

## Install

Same as the other plugins here: **File > Open** the `.wasm`, or `plugin.install` on the control
port, or drop it in the Additional Plug-ins Folder.

## Use

Select some paths, then **Object > Plug-ins > Round Coordinates**. Params:

- **grid** (0.01–100 pt): snap spacing.
- **roundHandles** (on by default): also snap each anchor's bezier handles, not just its point. Off
  keeps the curve shape closer to the original at the cost of leaving handle coordinates messy.

Verified with unit tests (`cargo test`, including a case that caught a real bug: using `Value`'s
`Index` instead of `get`/`get_mut` on a missing `"path"` key silently inserts a null one — fixed
by reading with `get_mut` throughout) and against `vectorcraft-plugins`' own `wasmi` host the same
way as the other plugins here.
