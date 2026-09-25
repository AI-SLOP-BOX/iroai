use iroai_core::{BlendMode, Color, Document};

#[test]
fn test_blend_mode_normal() {
    let base = Color::rgb(100, 100, 100);
    let src = Color::rgba(200, 200, 200, 128);
    let res = BlendMode::Normal.blend_pixel(base, src, 1.0);
    assert!((res.r as i32 - 150).abs() <= 2);
    assert!((res.g as i32 - 150).abs() <= 2);
    assert!((res.b as i32 - 150).abs() <= 2);
}

#[test]
fn test_blend_mode_multiply() {
    let base = Color::rgb(255, 128, 0);
    let src = Color::rgb(128, 255, 255);
    let res = BlendMode::Multiply.blend_pixel(base, src, 1.0);
    assert!((res.r as i32 - 128).abs() <= 2);
    assert!((res.g as i32 - 128).abs() <= 2);
    assert_eq!(res.b, 0);
}

#[test]
fn test_clipping_mask_composite() {
    let mut doc = Document::new(10, 10, "Clipping Test");
    if let Some(l) = doc.active_layer_mut() {
        l.buffer.clear();
        for y in 0..10 {
            for x in 0..5 {
                l.buffer.set_pixel(x, y, Color::rgb(255, 0, 0));
            }
        }
    }
    let _l1_id = doc.add_layer("Green Clipped");
    if let Some(l) = doc.active_layer_mut() {
        l.clipping_mask = true;
        l.buffer.fill(Color::rgb(0, 255, 0));
    }

    let comp = doc.composite();
    assert_eq!(comp.get_pixel(2, 2).unwrap(), Color::rgb(0, 255, 0));
    assert_eq!(comp.get_pixel(7, 7).unwrap(), Color::TRANSPARENT);
}

#[test]
fn test_psd_roundtrip() {
    let psd_path = std::env::temp_dir().join(format!("test_psd_{}.psd", uuid::Uuid::new_v4()));
    let mut doc = Document::new(20, 20, "PSD Test");
    if let Some(l) = doc.active_layer_mut() {
        l.name = "Background".to_string();
        l.buffer.fill(Color::rgb(255, 0, 0));
    }
    let _ = doc.add_layer("Blue Layer");
    if let Some(l) = doc.active_layer_mut() {
        l.opacity = 0.8;
        l.buffer.fill(Color::rgb(0, 0, 255));
    }

    iroai_core::PsdHandler::export_psd(&doc, &psd_path).expect("Failed to export PSD");
    let loaded = iroai_core::PsdHandler::load_psd(&psd_path).expect("Failed to load PSD");
    assert_eq!(loaded.width, 20);
    assert_eq!(loaded.height, 20);
    assert_eq!(loaded.layers.len(), 2);
    assert_eq!(loaded.layers[0].name, "Background");
    assert_eq!(loaded.layers[1].name, "Blue Layer");

    let _ = std::fs::remove_file(psd_path);
}
