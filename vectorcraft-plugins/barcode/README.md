# EAN-13 Barcode

A VectorCraft object-filter plug-in that generates a standard EAN-13 barcode as vector bars.
VectorCraft has no built-in barcode generation.

A QR code was the first idea, but the v1 plug-in ABI's dialog params are number/int/bool/choice
only — no free-text field — so there's no way to type an arbitrary payload into it. EAN-13 fits
because its content is just a number: give it the 12-digit code, it computes the check digit and
draws the bars.

## Build

```sh
rustup target add wasm32-unknown-unknown
./build.sh
```

## Install

Same as the other plugins here: **File > Open** the `.wasm`, or `plugin.install` on the control
port, or drop it in the Additional Plug-ins Folder.

## Use

**Object > Plug-ins > EAN-13 Barcode** (works with nothing selected — it's a generator). Params:

- **value** (0–999999999999): the 12-digit payload; the 13th (check) digit is computed.
- **moduleWidth** (0.25–10 pt): width of the narrowest bar.
- **height** (5–300 pt): bar height.

Verified with unit tests (`cargo test`, including against a real published EAN-13 — Nivea's
`4006381333931` — to check the parity/checksum tables) and against `vectorcraft-plugins`' own
`wasmi` host the same way as the Stipple plugin.

## What this isn't

No human-readable digits under the bars (a filter can only output path geometry, not text) and no
quiet-zone margin drawn (add blank space around it yourself when placing it). No QR, Code 128, or
other formats — see the ABI limitation above for why.
