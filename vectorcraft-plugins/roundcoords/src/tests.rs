use super::*;

#[test]
fn rounds_anchor_and_handles_to_grid() {
    let mut obj = json!({
        "id": 1, "type": "path",
        "path": {"subpaths": [{"closed": true, "anchors": [
            {"p": [10.2, 9.8], "in": [10.05, 9.6], "out": [10.4, 10.1]},
            {"p": [20.49, 0.0]}
        ]}]}
    });
    round_object(&mut obj, 1.0, true);
    let anchors = obj["path"]["subpaths"][0]["anchors"].as_array().unwrap();
    assert_eq!(anchors[0]["p"], json!([10.0, 10.0]));
    assert_eq!(anchors[0]["in"], json!([10.0, 10.0]));
    assert_eq!(anchors[0]["out"], json!([10.0, 10.0]));
    assert_eq!(anchors[1]["p"], json!([20.0, 0.0]));
}

#[test]
fn leaves_handles_alone_when_disabled() {
    let mut obj = json!({"path": {"subpaths": [{"closed": true, "anchors": [
        {"p": [10.2, 9.8], "in": [10.05, 9.6]}
    ]}]}});
    round_object(&mut obj, 1.0, false);
    let a = &obj["path"]["subpaths"][0]["anchors"][0];
    assert_eq!(a["p"], json!([10.0, 10.0]));
    assert_eq!(a["in"], json!([10.05, 9.6]), "handles must be untouched when roundHandles is false");
}

#[test]
fn larger_grid_snaps_to_coarser_steps() {
    let mut obj = json!({"path": {"subpaths": [{"closed": true, "anchors": [{"p": [12.0, 37.0]}]}]}});
    round_object(&mut obj, 10.0, true);
    assert_eq!(obj["path"]["subpaths"][0]["anchors"][0]["p"], json!([10.0, 40.0]));
}

#[test]
fn object_with_no_path_is_left_alone() {
    let mut obj = json!({"id": 1, "name": "no geometry here"});
    round_object(&mut obj, 1.0, true);
    assert_eq!(obj, json!({"id": 1, "name": "no geometry here"}));
}
