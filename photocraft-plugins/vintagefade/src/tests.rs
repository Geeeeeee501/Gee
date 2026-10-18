use super::*;
use serde_json::json;

#[test]
fn vignette_is_full_at_centre_and_darker_at_corner() {
    assert_eq!(vignette_factor(50.0, 50.0, 100.0, 100.0, 0.5), 1.0);
    let corner = vignette_factor(0.0, 0.0, 100.0, 100.0, 0.5);
    assert!(corner < 0.6 && corner > 0.0, "{corner}");
}

#[test]
fn fade_pulls_towards_mid_grey() {
    assert!((fade(0.0, 1.0) - 0.125).abs() < 1e-9, "black lifts towards grey");
    assert!((fade(1.0, 1.0) - 0.875).abs() < 1e-9, "white pulls down towards grey");
    assert_eq!(fade(0.3, 0.0), 0.3, "zero amount is a no-op");
}

#[test]
fn warmth_zero_is_a_no_op() {
    assert_eq!(warm_split_tone(0.2, 0.4, 0.6, 0.0), (0.2, 0.4, 0.6));
}

#[test]
fn warmth_warms_highlights_and_cools_shadows() {
    let (hr, _, hb) = warm_split_tone(0.9, 0.9, 0.9, 0.5); // a highlight
    let (sr, _, sb) = warm_split_tone(0.1, 0.1, 0.1, 0.5); // a shadow
    assert!(hr > 0.9, "highlight red should rise, got {hr}");
    assert!(hb < 0.9, "highlight blue should fall, got {hb}");
    assert!(sr < 0.1, "shadow red should fall, got {sr}");
    assert!(sb > 0.1, "shadow blue should rise (cooler), got {sb}");
}

#[test]
fn hash_noise_is_bounded_and_deterministic() {
    for (x, y) in [(0, 0), (123, 456), (-7, 99)] {
        let a = hash_noise(x, y);
        let b = hash_noise(x, y);
        assert_eq!(a, b, "same input must give the same noise");
        assert!((-1.0..=1.0).contains(&a), "{a}");
    }
    assert_ne!(hash_noise(0, 0), hash_noise(1, 0), "different pixels should (almost always) differ");
}

#[test]
fn pc_filter_neutral_params_leave_centre_pixel_almost_unchanged() {
    // 3x3 RGB image, no alpha; default-ish params but vignette/grain/fade/warmth all 0 so the
    // centre pixel (which sees vignette factor 1 by construction) should pass through untouched.
    let mut buf = vec![0.5f32; 3 * 3 * 3];
    let params = json!({
        "fade": 0.0, "warmth": 0.0, "vignette": 0.0, "grain": 0.0,
        "_image": {"x": 0, "y": 0, "canvasWidth": 3.0, "canvasHeight": 3.0}
    });
    let params_bytes = serde_json::to_vec(&params).unwrap();
    let format = 8u32 | (3u32 << 16); // depth 8, mode RGB, no alpha bit
    let rc = unsafe {
        pc_filter(buf.as_mut_ptr(), (buf.len() * 4) as u32, 3, 3, 3, format, params_bytes.as_ptr(), params_bytes.len() as u32)
    };
    assert_eq!(rc, 0);
    // Centre pixel (index 4) is at the exact canvas centre: vignette factor is exactly 1.
    let centre = &buf[4 * 3..4 * 3 + 3];
    assert!((centre[0] - 0.5).abs() < 1e-6, "{centre:?}");
}
