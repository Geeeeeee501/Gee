//! Color Adjust: a VectorCraft object filter plug-in (plug-in ABI v1, see
//! `storytold/vectorcraft`'s `docs/plugins.md`).
//!
//! Shifts hue, scales saturation and shifts brightness/lightness of every colour of the selection
//! (solid fills and strokes, gradient stops, freeform gradient points), each in its own colour
//! model. A stand-in for Astute Graphics' "Phantasm" live HSB adjustment; VectorCraft's own Edit >
//! Edit Colors only exposes per-channel (R/G/B or C/M/Y/K) balance, not hue/saturation/brightness.
//! Walks the JSON the same way the Desaturate example plug-in does. Only `serde_json` is used: no
//! imports, no WASI.

use serde_json::{Value, json};

const MANIFEST: &str = r#"{
  "id": "org.vectorcraft.community.coloradjust",
  "name": "Hue/Saturation/Brightness",
  "version": "1.0.0",
  "kind": "filter",
  "author": "Community",
  "description": "Shifts hue, scales saturation and shifts brightness of the selection's colours.",
  "params": {
    "hue": {"type": "number", "min": -180, "max": 180, "default": 0},
    "saturation": {"type": "number", "min": -100, "max": 100, "default": 0},
    "brightness": {"type": "number", "min": -100, "max": 100, "default": 0}
  }
}"#;

#[unsafe(no_mangle)]
pub extern "C" fn vc_abi_version() -> u32 {
    1
}

/// `len << 32 | ptr` of the UTF-8 manifest JSON.
#[unsafe(no_mangle)]
pub extern "C" fn vc_manifest() -> u64 {
    pack(MANIFEST.as_bytes())
}

/// A block of `size` bytes the host writes the input and parameters into. Never freed: every run
/// gets a fresh instance.
#[unsafe(no_mangle)]
pub extern "C" fn vc_alloc(size: u32) -> u32 {
    let mut block = Vec::<u8>::with_capacity(size as usize);
    let ptr = block.as_mut_ptr();
    std::mem::forget(block);
    ptr as u32
}

/// Runs the filter: `input` holds `{"objects": [...]}`, `params` the parameters. Returns the
/// output JSON as `len << 32 | ptr`, or a negative error code.
///
/// # Safety
/// The host guarantees both blocks came from `vc_alloc` and hold `*_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vc_run(input: *const u8, input_len: u32, params: *const u8, params_len: u32) -> i64 {
    // SAFETY: see the function's contract.
    let (input, params) = unsafe { (std::slice::from_raw_parts(input, input_len as usize), std::slice::from_raw_parts(params, params_len as usize)) };
    let (Ok(mut doc), Ok(params)) = (serde_json::from_slice::<Value>(input), serde_json::from_slice::<Value>(params)) else { return -1 };
    let adj = Adjust {
        hue: params["hue"].as_f64().unwrap_or(0.0),
        sat: (params["saturation"].as_f64().unwrap_or(0.0) / 100.0).clamp(-1.0, 1.0),
        bright: (params["brightness"].as_f64().unwrap_or(0.0) / 100.0).clamp(-1.0, 1.0),
    };
    if doc["objects"].as_array().is_none_or(|o| o.is_empty()) {
        return output(&json!({"error": "Select some paths to adjust."}));
    }
    walk(&mut doc, &adj);
    output(&json!({"objects": doc["objects"]}))
}

struct Adjust {
    hue: f64,
    sat: f64,
    bright: f64,
}

/// Every colour (an object with a `model`) inside `v`, adjusted.
fn walk(v: &mut Value, adj: &Adjust) {
    match v {
        Value::Object(o) => {
            let model = o.get("model").and_then(Value::as_str).map(str::to_string);
            let get = |key: &str| o.get(key).and_then(Value::as_f64).unwrap_or(0.0);
            let new: Option<Vec<(&str, f64)>> = match model.as_deref() {
                Some("rgb") => {
                    let (r, g, b) = adjust_rgb((get("r"), get("g"), get("b")), adj);
                    Some(vec![("r", r), ("g", g), ("b", b)])
                }
                Some("gray") => Some(vec![("k", (get("k") - adj.bright).clamp(0.0, 1.0))]),
                Some("lab") => {
                    let (l, a, b) = adjust_lab((get("l"), get("a"), get("b")), adj);
                    Some(vec![("l", l), ("a", a), ("b", b)])
                }
                Some("cmyk") => {
                    let (c, m, y, k) = adjust_cmyk((get("c"), get("m"), get("y"), get("k")), adj);
                    Some(vec![("c", c), ("m", m), ("y", y), ("k", k)])
                }
                _ => None,
            };
            match new {
                Some(new) => {
                    for (key, x) in new {
                        o.insert(key.into(), json!(x));
                    }
                }
                None => o.values_mut().for_each(|c| walk(c, adj)),
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|c| walk(c, adj)),
        _ => {}
    }
}

/// `(r, g, b)` in 0..=1, hue-rotated, saturation-scaled and lightness-shifted via HSL.
fn adjust_rgb((r, g, b): (f64, f64, f64), adj: &Adjust) -> (f64, f64, f64) {
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let h = (h + adj.hue).rem_euclid(360.0);
    let s = (s * (1.0 + adj.sat)).clamp(0.0, 1.0);
    let l = (l + adj.bright).clamp(0.0, 1.0);
    hsl_to_rgb(h, s, l)
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-9 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r1 + m, g1 + m, b1 + m)
}

/// `(l, a, b)` Lab, brightness shifting `l`, hue rotating and saturation scaling the chroma in the
/// `a`/`b` plane.
fn adjust_lab((l, a, b): (f64, f64, f64), adj: &Adjust) -> (f64, f64, f64) {
    let l = (l + adj.bright * 100.0).clamp(0.0, 100.0);
    let chroma = (a * a + b * b).sqrt() * (1.0 + adj.sat).max(0.0);
    let angle = a.atan2(b) + adj.hue.to_radians();
    (l, chroma * angle.sin(), chroma * angle.cos())
}

/// `(c, m, y, k)` 0..=1 ink, adjusted by converting the `c`/`m`/`y` tint to RGB at the current `k`,
/// adjusting that RGB, then solving back for `c`/`m`/`y` at the same `k` (so pure black, `k == 1`,
/// is left alone: hue and brightness have nothing to act on there, same as a human expects).
fn adjust_cmyk((c, m, y, k): (f64, f64, f64, f64), adj: &Adjust) -> (f64, f64, f64, f64) {
    if k >= 1.0 - 1e-9 {
        return (c, m, y, k);
    }
    let shade = 1.0 - k;
    let (r, g, b) = adjust_rgb(((1.0 - c) * shade, (1.0 - m) * shade, (1.0 - y) * shade), adj);
    (1.0 - (r / shade).min(1.0), 1.0 - (g / shade).min(1.0), 1.0 - (b / shade).min(1.0), k)
}

/// Keeps `v` serialised in memory for the host to read, as `len << 32 | ptr`.
fn output(v: &Value) -> i64 {
    let Ok(bytes) = serde_json::to_vec(v) else { return -2 };
    let bytes = bytes.leak();
    pack(bytes) as i64
}

fn pack(bytes: &[u8]) -> u64 {
    ((bytes.len() as u64) << 32) | bytes.as_ptr() as u64
}

#[cfg(test)]
mod tests;
