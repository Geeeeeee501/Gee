//! Vintage Fade: a PhotoCraft filter plug-in (plug-in ABI v1, see `docs/plugins.md`).
//!
//! A one-click "vintage/film" recipe — faded contrast, a warm-highlight/cool-shadow split tone,
//! grain and a corner vignette — packaged as a single filter instead of stacking four adjustment
//! layers by hand. PhotoCraft already has every individual Photoshop adjustment and filter this
//! would otherwise be built from (Curves, Color Balance, Add Noise, Lens Correction's vignette);
//! the plug-in's value is the packaged, tasteful-by-default combination, the same way most real
//! third-party Photoshop filter packs (Nik Color Efex, Alien Skin Exposure) are curated recipes
//! rather than new pixel math. Only `serde_json` is used: no imports, no WASI.

use serde_json::Value;

const MANIFEST: &str = r#"{
  "id": "org.photocraft.community.vintagefade",
  "name": "Vintage Fade",
  "version": "1.0.0",
  "kind": "filter",
  "author": "Community",
  "description": "Faded contrast, warm/cool split tone, grain and a corner vignette, in one filter.",
  "params": {
    "fade": {"type": "number", "min": 0, "max": 100, "default": 40},
    "warmth": {"type": "number", "min": -100, "max": 100, "default": 30},
    "vignette": {"type": "number", "min": 0, "max": 100, "default": 35},
    "grain": {"type": "number", "min": 0, "max": 100, "default": 15}
  },
  "overlap": 0,
  "area": "canvas"
}"#;

/// Bit 8 of `format`: the last channel is (straight) alpha.
const HAS_ALPHA: u32 = 1 << 8;
/// Bits 16-23 of `format`: the colour mode, PSD-numbered. 3 = RGB.
const MODE_RGB: u32 = 3;

#[derive(Clone, Copy)]
struct Params {
    fade: f64,
    warmth: f64,
    vignette: f64,
    grain: f64,
}

#[unsafe(no_mangle)]
pub extern "C" fn pc_abi_version() -> u32 {
    1
}

/// `len << 32 | ptr` of the UTF-8 manifest JSON.
#[unsafe(no_mangle)]
pub extern "C" fn pc_manifest() -> u64 {
    ((MANIFEST.len() as u64) << 32) | MANIFEST.as_ptr() as u64
}

/// A block of `size` bytes the host may write into. Never freed: every band gets a fresh
/// instance.
#[unsafe(no_mangle)]
pub extern "C" fn pc_alloc(size: u32) -> u32 {
    let mut block = Vec::<u8>::with_capacity(size as usize);
    let ptr = block.as_mut_ptr();
    std::mem::forget(block);
    ptr as u32
}

/// Filters `width * height` pixels of `channels` interleaved `f32` samples at `buf` in place.
/// `0` = success, anything else is an error code.
///
/// # Safety
/// The host guarantees `buf..buf + buf_len` is a block it got from `pc_alloc`, holding
/// `buf_len / 4` samples, and that `params..params + params_len` holds `params_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pc_filter(
    buf: *mut f32,
    buf_len: u32,
    width: u32,
    height: u32,
    channels: u32,
    format: u32,
    params: *const u8,
    params_len: u32,
) -> i32 {
    if channels == 0 {
        return 1;
    }
    // SAFETY: see the function's contract.
    let (px, params_bytes) =
        unsafe { (std::slice::from_raw_parts_mut(buf, buf_len as usize / 4), std::slice::from_raw_parts(params, params_len as usize)) };
    let Ok(v) = serde_json::from_slice::<Value>(params_bytes) else { return 2 };
    let p = Params {
        fade: v["fade"].as_f64().unwrap_or(40.0) / 100.0,
        warmth: v["warmth"].as_f64().unwrap_or(30.0) / 100.0,
        vignette: v["vignette"].as_f64().unwrap_or(35.0) / 100.0,
        grain: v["grain"].as_f64().unwrap_or(15.0) / 100.0,
    };
    let img = &v["_image"];
    let (ox, oy) = (img["x"].as_i64().unwrap_or(0), img["y"].as_i64().unwrap_or(0));
    let (cw, ch) = (img["canvasWidth"].as_f64().unwrap_or(width as f64), img["canvasHeight"].as_f64().unwrap_or(height as f64));
    let mode = (format >> 16) & 0xFF;
    let alpha = format & HAS_ALPHA != 0;
    let n = channels as usize;
    let colour = if alpha { n - 1 } else { n };

    for (i, pixel) in px.chunks_exact_mut(n).enumerate() {
        if alpha && pixel[colour] <= 0.0 {
            continue;
        }
        let (col, row) = (i as u32 % width, i as u32 / width);
        let vig = vignette_factor((ox + col as i64) as f64, (oy + row as i64) as f64, cw, ch, p.vignette);
        let noise = hash_noise(ox + col as i64, oy + row as i64) * p.grain * 0.08;
        if mode == MODE_RGB && colour >= 3 {
            let (r, g, b) = warm_split_tone(pixel[0] as f64, pixel[1] as f64, pixel[2] as f64, p.warmth);
            pixel[0] = clamp01(fade(r, p.fade) + noise) as f32 * vig as f32;
            pixel[1] = clamp01(fade(g, p.fade) + noise) as f32 * vig as f32;
            pixel[2] = clamp01(fade(b, p.fade) + noise) as f32 * vig as f32;
        } else {
            for v in &mut pixel[..colour] {
                *v = (clamp01(fade(*v as f64, p.fade) + noise) * vig) as f32;
            }
        }
    }
    0
}

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

/// Lifts shadows and pulls down highlights towards mid-grey: classic "faded film" low contrast.
fn fade(v: f64, amount: f64) -> f64 {
    v + amount * 0.25 * (0.5 - v)
}

/// A simple split tone: highlights lean warm (more red, less blue), shadows lean cool, scaled by
/// `warmth` (negative reverses it: warm shadows, cool highlights).
fn warm_split_tone(r: f64, g: f64, b: f64, warmth: f64) -> (f64, f64, f64) {
    let lum = (r + g + b) / 3.0;
    let t = lum - 0.5; // -0.5 (black) .. 0.5 (white)
    (r + warmth * 0.15 * t, g, b - warmth * 0.15 * t)
}

/// 1 at the canvas centre, down to `1 - amount` at the far corner.
fn vignette_factor(abs_x: f64, abs_y: f64, canvas_w: f64, canvas_h: f64, amount: f64) -> f64 {
    if canvas_w <= 0.0 || canvas_h <= 0.0 {
        return 1.0;
    }
    let dx = (abs_x - canvas_w / 2.0) / (canvas_w / 2.0);
    let dy = (abs_y - canvas_h / 2.0) / (canvas_h / 2.0);
    let r = (dx * dx + dy * dy).sqrt().min(1.414) / 1.414;
    1.0 - amount * r * r
}

/// A deterministic pseudo-random value in `-1..=1` for one absolute pixel coordinate: no state,
/// so it's identical in every band regardless of how the host tiles the image.
fn hash_noise(x: i64, y: i64) -> f64 {
    let mut h = (x as u64).wrapping_mul(374_761_393).wrapping_add((y as u64).wrapping_mul(668_265_263));
    h ^= h >> 13;
    h = h.wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h as f64 / u64::MAX as f64) * 2.0 - 1.0
}

#[cfg(test)]
mod tests;
