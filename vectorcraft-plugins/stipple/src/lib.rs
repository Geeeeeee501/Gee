//! Stipple: a VectorCraft object filter plug-in (plug-in ABI v1, see
//! `storytold/vectorcraft`'s `docs/plugins.md`).
//!
//! Fills the selected path(s)/compound path(s) with a jittered grid of small solid dots clipped to
//! the shape's own fill area (honouring its fill rule), coloured with the object's own fill colour.
//! A stand-in for Astute Graphics' "Stipplism", which VectorCraft has no built-in equivalent of.
//! Only `serde_json` is used: no imports, no WASI.

use serde_json::{Value, json};

const MANIFEST: &str = r#"{
  "id": "org.vectorcraft.community.stipple",
  "name": "Stipple",
  "version": "1.0.0",
  "kind": "filter",
  "author": "Community",
  "description": "Fills the selection with a jittered grid of dots clipped to its shape, in its own fill colour.",
  "params": {
    "spacing": {"type": "number", "min": 1, "max": 200, "default": 10},
    "dotSize": {"type": "number", "min": 5, "max": 100, "default": 45},
    "jitter":  {"type": "number", "min": 0, "max": 100, "default": 55}
  }
}"#;

/// Dots above this are rejected rather than built, so a tiny `spacing` on a large shape fails fast
/// instead of producing a huge, slow document.
const MAX_DOTS: usize = 20_000;
/// Bezier flattening steps per path segment, for the point-in-shape test.
const FLATTEN_STEPS: usize = 10;
const KAPPA: f64 = 0.552_284_749_8;

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
    let (Ok(doc), Ok(params)) = (serde_json::from_slice::<Value>(input), serde_json::from_slice::<Value>(params)) else { return -1 };
    let Some(objects) = doc["objects"].as_array() else { return output(&json!({"error": "Select a filled path to stipple."})) };
    if objects.is_empty() {
        return output(&json!({"error": "Select a filled path to stipple."}));
    }
    let spacing = params["spacing"].as_f64().unwrap_or(10.0).max(0.5);
    let dot_r = spacing * (params["dotSize"].as_f64().unwrap_or(45.0) / 100.0).clamp(0.01, 1.0) / 2.0;
    let jitter = (params["jitter"].as_f64().unwrap_or(55.0) / 100.0).clamp(0.0, 1.0);

    let mut out = Vec::with_capacity(objects.len());
    let mut total_dots = 0usize;
    for obj in objects {
        match stipple_one(obj, spacing, dot_r, jitter) {
            Ok(Some(new_obj)) => {
                total_dots += new_obj["path"]["subpaths"].as_array().map_or(0, Vec::len);
                out.push(new_obj);
            }
            Ok(None) => {}
            Err(msg) => return output(&json!({"error": msg})),
        }
    }
    if total_dots == 0 {
        return output(&json!({"error": "No fill area found. Try a smaller spacing, or check the selection has a solid fill."}));
    }
    output(&json!({"objects": out}))
}

/// One input object stippled, or `None` if it has no usable fill area (skipped, not an error), or
/// `Err` if the grid would be too dense.
fn stipple_one(obj: &Value, spacing: f64, dot_r: f64, jitter: f64) -> Result<Option<Value>, String> {
    let Some(subpaths) = obj["path"]["subpaths"].as_array() else { return Ok(None) };
    let polylines: Vec<Vec<(f64, f64)>> = subpaths.iter().filter_map(flatten_subpath).collect();
    if polylines.is_empty() {
        return Ok(None);
    }
    let evenodd = obj["fillRule"].as_str() == Some("evenodd");
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for pl in &polylines {
        for &(x, y) in pl {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if !(x0.is_finite() && y0.is_finite() && x1 > x0 && y1 > y0) {
        return Ok(None);
    }
    let (cols, rows) = (((x1 - x0) / spacing).ceil() as usize + 1, ((y1 - y0) / spacing).ceil() as usize + 1);
    if cols.saturating_mul(rows) > MAX_DOTS * 4 {
        return Err("Spacing is too small for this shape's size: increase it.".into());
    }

    let color = fill_color(obj);
    let mut rng = Rng(obj["id"].as_u64().unwrap_or(1).wrapping_mul(2_654_435_761).max(1));
    let mut circles = Vec::new();
    let half = spacing * jitter * 0.5;
    let mut gy = y0;
    while gy <= y1 {
        let mut gx = x0;
        while gx <= x1 {
            let px = gx + rng.jitter(half);
            let py = gy + rng.jitter(half);
            if inside(&polylines, evenodd, px, py) {
                circles.push(circle_subpath(px, py, dot_r));
                if circles.len() > MAX_DOTS {
                    return Err("Spacing is too small for this shape's size: increase it.".into());
                }
            }
            gx += spacing;
        }
        gy += spacing;
    }
    if circles.is_empty() {
        return Ok(None);
    }
    Ok(Some(json!({
        "type": if circles.len() > 1 { "compound" } else { "path" },
        "fillRule": "nonzero",
        "path": {"subpaths": circles},
        "fills": [{"type": "solid", "color": color}],
        "strokes": [],
        "opacity": obj["opacity"].as_f64().unwrap_or(1.0),
    })))
}

/// The object's own fill colour (its bottom solid fill), or black if it has none.
fn fill_color(obj: &Value) -> Value {
    obj["fills"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|f| (f["type"] == "solid").then(|| f["color"].clone()))
        .unwrap_or_else(|| json!({"model": "rgb", "r": 0.0, "g": 0.0, "b": 0.0}))
}

/// A subpath's anchors flattened to a closed polyline (fills always close the shape).
fn flatten_subpath(subpath: &Value) -> Option<Vec<(f64, f64)>> {
    let anchors = subpath["anchors"].as_array()?;
    if anchors.len() < 2 {
        return None;
    }
    let pt = |a: &Value, key: &str| -> Option<(f64, f64)> {
        let p = a.get(key)?.as_array()?;
        Some((p.first()?.as_f64()?, p.get(1)?.as_f64()?))
    };
    let mut out = Vec::new();
    let n = anchors.len();
    for i in 0..n {
        let a = &anchors[i];
        let b = &anchors[(i + 1) % n];
        let p0 = pt(a, "p")?;
        let p3 = pt(b, "p")?;
        let p1 = pt(a, "out").unwrap_or(p0);
        let p2 = pt(b, "in").unwrap_or(p3);
        for s in 0..FLATTEN_STEPS {
            out.push(cubic_at(p0, p1, p2, p3, s as f64 / FLATTEN_STEPS as f64));
        }
    }
    Some(out)
}

fn cubic_at(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1)
}

/// Ray-casting point-in-shape test across every subpath's flattened polyline combined, by the
/// object's own fill rule.
fn inside(polylines: &[Vec<(f64, f64)>], evenodd: bool, px: f64, py: f64) -> bool {
    let (mut signed, mut count) = (0i32, 0u32);
    for pl in polylines {
        let n = pl.len();
        for i in 0..n {
            let (x1, y1) = pl[i];
            let (x2, y2) = pl[(i + 1) % n];
            if (y1 <= py) != (y2 <= py) {
                let t = (py - y1) / (y2 - y1);
                if x1 + t * (x2 - x1) > px {
                    count += 1;
                    signed += if y2 > y1 { 1 } else { -1 };
                }
            }
        }
    }
    if evenodd { count % 2 == 1 } else { signed != 0 }
}

/// A closed circular subpath (4 anchors, bezier-approximated) around `(cx, cy)` with radius `r`.
fn circle_subpath(cx: f64, cy: f64, r: f64) -> Value {
    let k = r * KAPPA;
    let anchor = |p: (f64, f64), h_in: (f64, f64), h_out: (f64, f64)| json!({"p": [p.0, p.1], "in": [h_in.0, h_in.1], "out": [h_out.0, h_out.1]});
    json!({"closed": true, "anchors": [
        anchor((cx, cy - r), (cx - k, cy - r), (cx + k, cy - r)),
        anchor((cx + r, cy), (cx + r, cy - k), (cx + r, cy + k)),
        anchor((cx, cy + r), (cx + k, cy + r), (cx - k, cy + r)),
        anchor((cx - r, cy), (cx - r, cy + k), (cx - r, cy - k)),
    ]})
}

/// A tiny deterministic xorshift RNG, seeded per object so the same document stipples the same way
/// every time it's run (no system randomness is available to a plug-in).
struct Rng(u64);

impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A jitter offset uniform in `[-half, half]`.
    fn jitter(&mut self, half: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * half
    }
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
