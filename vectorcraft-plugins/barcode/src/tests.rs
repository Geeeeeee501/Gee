use super::*;

#[test]
fn known_ean13_check_digit() {
    // 4006381333931 is a real, published EAN-13 (Nivea); payload 400638133393, check digit 1.
    let d = ean13_digits(400_638_133_393);
    assert_eq!(d, [4, 0, 0, 6, 3, 8, 1, 3, 3, 3, 9, 3, 1]);
}

#[test]
fn encoded_bits_are_95_modules() {
    let d = ean13_digits(400_638_133_393);
    assert_eq!(encode_bits(&d).len(), 95);
}

#[test]
fn bars_merge_into_contiguous_rects() {
    let rects = bars_to_rects("1101", 2.0, 10.0);
    // "11" merges into one rect, "0" is a gap, "1" is its own rect.
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0]["anchors"][0]["p"], json!([0.0, 0.0]));
    assert_eq!(rects[0]["anchors"][1]["p"], json!([4.0, 0.0]), "first run is 2 modules wide");
    assert_eq!(rects[1]["anchors"][0]["p"], json!([6.0, 0.0]), "second run starts after the gap");
}
