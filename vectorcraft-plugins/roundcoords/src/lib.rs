//! Round Coordinates: a VectorCraft object filter plug-in (plug-in ABI v1, see
//! `storytold/vectorcraft`'s `docs/plugins.md`).
//!
//! Snaps every anchor (and, optionally, its handles) of the selected paths/compound paths to the
//! nearest multiple of a grid size. VectorCraft has interactive Snap to Grid/Pixel for dragging,
//! but no batch command to clean up coordinates already on the page — useful before sending art to
//! a cutter or plotter, where stray sub-unit jitter from an import or a trace causes tiny,
//! unwanted segments. Only `serde_json` is used: no imports, no WASI.

use serde_json::{Value, json};

const MANIFEST: &str = r#"{
  "id": "org.vectorcraft.community.roundcoords",
  "name": "Round Coordinates",
  "version": "1.0.0",
  "kind": "filter",
  "author": "Community",
  "description": "Snaps every anchor (and optionally its handles) to the nearest multiple of a grid size.",
  "params": {
    "grid": {"type": "number", "min": 0.01, "max": 100, "default": 1},
    "roundHandles": {"type": "bool", "default": true}
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
    let Some(objects) = doc["objects"].as_array() else { return output(&json!({"error": "Select some paths to round."})) };
    if objects.is_empty() {
        return output(&json!({"error": "Select some paths to round."}));
    }
    let grid = params["grid"].as_f64().unwrap_or(1.0).max(1e-6);
    let round_handles = params["roundHandles"].as_bool().unwrap_or(true);

    for obj in doc["objects"].as_array_mut().unwrap_or(&mut Vec::new()) {
        round_object(obj, grid, round_handles);
    }
    output(&json!({"objects": doc["objects"]}))
}

fn round_object(obj: &mut Value, grid: f64, round_handles: bool) {
    let Some(subpaths) = obj.get_mut("path").and_then(|p| p.get_mut("subpaths")).and_then(Value::as_array_mut) else { return };
    for sp in subpaths {
        let Some(anchors) = sp["anchors"].as_array_mut() else { continue };
        for a in anchors {
            round_point(a, "p", grid);
            if round_handles {
                round_point(a, "in", grid);
                round_point(a, "out", grid);
            }
        }
    }
}

fn round_point(anchor: &mut Value, key: &str, grid: f64) {
    let Some(p) = anchor.get_mut(key).and_then(Value::as_array_mut) else { return };
    for coord in p.iter_mut() {
        if let Some(v) = coord.as_f64() {
            *coord = json!((v / grid).round() * grid);
        }
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

#[cfg(test)]
mod tests;
