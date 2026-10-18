//! Barcode: a VectorCraft object filter plug-in (plug-in ABI v1, see
//! `storytold/vectorcraft`'s `docs/plugins.md`).
//!
//! Generates a standard EAN-13 barcode as vector bars from a 12-digit number (the check digit is
//! computed). A generator: the selection is ignored, new objects are added. VectorCraft has no
//! built-in barcode generation. Only `serde_json` is used: no imports, no WASI.
//!
//! The v1 ABI's manifest params are number/int/bool/choice only — no free text — so there is no
//! way to type arbitrary string content (a QR code's payload, say) through the generated dialog.
//! A plain numeric code is the barcode format that fits: EAN-13, the standard retail barcode.

use serde_json::{Value, json};

const MANIFEST: &str = r#"{
  "id": "org.vectorcraft.community.barcode",
  "name": "EAN-13 Barcode",
  "version": "1.0.0",
  "kind": "filter",
  "author": "Community",
  "description": "Generates an EAN-13 barcode (12 digits + computed check digit) as vector bars.",
  "params": {
    "value": {"type": "int", "min": 0, "max": 999999999999, "default": 123456789012},
    "moduleWidth": {"type": "number", "min": 0.25, "max": 10, "default": 1},
    "height": {"type": "number", "min": 5, "max": 300, "default": 60}
  }
}"#;

/// 7-module patterns for digits 0-9, `1` = bar, `0` = space.
const L_CODE: [&str; 10] =
    ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"];
const G_CODE: [&str; 10] =
    ["0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001", "0001001", "0010111"];
/// Which of the first six digits (after the number-system digit) use `G_CODE` instead of
/// `L_CODE`, indexed by the number-system digit (0-9); `R_CODE` (the bit-complement of `L_CODE`)
/// is always used for the last six.
const PARITY: [&str; 10] = ["LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL"];

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

/// Runs the filter: `params` holds this generator's parameters (the selection is ignored).
/// Returns the output JSON as `len << 32 | ptr`, or a negative error code.
///
/// # Safety
/// The host guarantees both blocks came from `vc_alloc` and hold `*_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vc_run(_input: *const u8, _input_len: u32, params: *const u8, params_len: u32) -> i64 {
    // SAFETY: see the function's contract.
    let params = unsafe { std::slice::from_raw_parts(params, params_len as usize) };
    let Ok(params) = serde_json::from_slice::<Value>(params) else { return -1 };
    let value = params["value"].as_u64().unwrap_or(123_456_789_012).min(999_999_999_999);
    let module_w = params["moduleWidth"].as_f64().unwrap_or(1.0).max(0.1);
    let height = params["height"].as_f64().unwrap_or(60.0).max(1.0);

    let digits = ean13_digits(value);
    let bits = encode_bits(&digits);
    let rects = bars_to_rects(&bits, module_w, height);
    output(&json!({"objects": [{
        "type": if rects.len() > 1 { "compound" } else { "path" },
        "fillRule": "nonzero",
        "path": {"subpaths": rects},
        "fills": [{"type": "solid", "color": {"model": "rgb", "r": 0.0, "g": 0.0, "b": 0.0}}],
        "strokes": [],
        "opacity": 1.0,
    }]}))
}

/// The 13 digits (12-digit `value`, zero-padded, plus the computed EAN-13 check digit).
fn ean13_digits(value: u64) -> [u8; 13] {
    let mut d = [0u8; 13];
    let mut v = value % 1_000_000_000_000;
    for i in (0..12).rev() {
        d[i] = (v % 10) as u8;
        v /= 10;
    }
    let sum: u32 = (0..12).map(|i| d[i] as u32 * if i % 2 == 0 { 1 } else { 3 }).sum();
    d[12] = ((10 - (sum % 10)) % 10) as u8;
    d
}

/// The full 95-module bit string (start guard, 6 left digits, centre guard, 6 right digits, end
/// guard), `'1'` a bar and `'0'` a space.
fn encode_bits(d: &[u8; 13]) -> String {
    let parity = PARITY[d[0] as usize];
    let mut s = String::from("101");
    for (i, p) in parity.chars().enumerate() {
        s += if p == 'L' { L_CODE[d[1 + i] as usize] } else { G_CODE[d[1 + i] as usize] };
    }
    s += "01010";
    for i in 7..13 {
        s += &complement(L_CODE[d[i] as usize]);
    }
    s += "101";
    s
}

fn complement(pattern: &str) -> String {
    pattern.chars().map(|c| if c == '1' { '0' } else { '1' }).collect()
}

/// `bits` turned into one rectangle per run of consecutive bars, each `module_w` wide per module
/// and `height` tall, left edge at x = 0.
fn bars_to_rects(bits: &str, module_w: f64, height: f64) -> Vec<Value> {
    let mut rects = Vec::new();
    let mut run_start: Option<usize> = None;
    let bytes: Vec<u8> = bits.bytes().collect();
    for (i, &b) in bytes.iter().chain([&b'0']).enumerate() {
        let bar = b == b'1';
        match (bar, run_start) {
            (true, None) => run_start = Some(i),
            (false, Some(start)) => {
                rects.push(rect(start as f64 * module_w, 0.0, (i - start) as f64 * module_w, height));
                run_start = None;
            }
            _ => {}
        }
    }
    rects
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Value {
    let anchor = |p: (f64, f64)| json!({"p": [p.0, p.1]});
    json!({"closed": true, "anchors": [anchor((x, y)), anchor((x + w, y)), anchor((x + w, y + h)), anchor((x, y + h))]})
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
