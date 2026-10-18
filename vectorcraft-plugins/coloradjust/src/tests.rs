use super::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn rgb_hsl_roundtrip() {
    for (r, g, b) in [(0.8, 0.2, 0.1), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0), (0.3, 0.3, 0.3), (0.1, 0.9, 0.4)] {
        let (h, s, l) = rgb_to_hsl(r, g, b);
        let (r2, g2, b2) = hsl_to_rgb(h, s, l);
        assert!(close(r, r2) && close(g, g2) && close(b, b2), "{r},{g},{b} -> {r2},{g2},{b2}");
    }
}

#[test]
fn hue_rotate_turns_red_towards_green() {
    let noop = Adjust { hue: 0.0, sat: 0.0, bright: 0.0 };
    let rotated = Adjust { hue: 120.0, sat: 0.0, bright: 0.0 };
    assert_eq!(adjust_rgb((1.0, 0.0, 0.0), &noop), (1.0, 0.0, 0.0));
    let (r, g, b) = adjust_rgb((1.0, 0.0, 0.0), &rotated);
    assert!(close(r, 0.0) && close(g, 1.0) && close(b, 0.0), "pure red +120deg hue should be pure green, got ({r}, {g}, {b})");
}

#[test]
fn brightness_and_saturation_clamp() {
    let blown_out = Adjust { hue: 0.0, sat: 100.0, bright: 100.0 };
    let (r, g, b) = adjust_rgb((0.5, 0.3, 0.3), &blown_out);
    for c in [r, g, b] {
        assert!((0.0..=1.0).contains(&c), "{c} out of range");
    }
}

#[test]
fn pure_black_cmyk_is_untouched() {
    let adj = Adjust { hue: 90.0, sat: 50.0, bright: 50.0 };
    assert_eq!(adjust_cmyk((0.2, 0.3, 0.4, 1.0), &adj), (0.2, 0.3, 0.4, 1.0));
}

#[test]
fn lab_hue_rotate_is_reversible() {
    let adj = Adjust { hue: 45.0, sat: 0.0, bright: 0.0 };
    let back = Adjust { hue: -45.0, sat: 0.0, bright: 0.0 };
    let (l, a, b) = adjust_lab((50.0, 20.0, -10.0), &adj);
    let (l2, a2, b2) = adjust_lab((l, a, b), &back);
    assert!(close(l2, 50.0) && close(a2, 20.0) && close(b2, -10.0), "got ({l2}, {a2}, {b2})");
}
