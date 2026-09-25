use iroai_core::{
    BezierEngine, Color, ColorCmyk, CmykManager, PathPoint, PixelBuffer, TilePyramid,
};

#[test]
fn test_cmyk_rgb_roundtrip() {
    let original = Color::rgb(220, 50, 80);
    let cmyk = ColorCmyk::from_rgb(original);
    let recon = cmyk.to_rgb(255);

    // RGB -> CMYK -> RGB color fidelity check
    assert!((original.r as i32 - recon.r as i32).abs() <= 5);
    assert!((original.g as i32 - recon.g as i32).abs() <= 5);
    assert!((original.b as i32 - recon.b as i32).abs() <= 5);
}

#[test]
fn test_tac_overrun_detection() {
    let rich_black = ColorCmyk::new(1.0, 1.0, 1.0, 1.0);
    assert_eq!(rich_black.total_ink_coverage(), 400.0);

    let mut buf = PixelBuffer::new(2, 2);
    // Dark mixed color that yields high TAC
    buf.set_pixel(0, 0, Color::rgb(5, 5, 5));
    // Moderate light color (low TAC)
    buf.set_pixel(1, 1, Color::rgb(220, 220, 220));

    let overrun = CmykManager::detect_tac_overrun(&buf, 90.0);
    assert!(overrun[0]);
    assert!(!overrun[3]);
}

#[test]
fn test_tile_pyramid_multiscale() {
    let mut buf = PixelBuffer::new(256, 256);
    buf.fill(Color::rgb(120, 150, 180));

    let pyramid = TilePyramid::build(&buf, 4);
    assert_eq!(pyramid.levels.len(), 4);
    assert_eq!(pyramid.levels[0].width, 256);
    assert_eq!(pyramid.levels[1].width, 128);
    assert_eq!(pyramid.levels[2].width, 64);
    assert_eq!(pyramid.levels[3].width, 32);

    // Verify sampling correctly retrieves downscaled level for low zoom
    let low_zoom_buf = pyramid.get_level_for_zoom(0.1);
    assert_eq!(low_zoom_buf.width, 32);
}

#[test]
fn test_bezier_curve_evaluation_and_raster() {
    let p_start = PathPoint {
        anchor: (10.0, 10.0),
        handle_in: None,
        handle_out: Some((20.0, 50.0)),
    };
    let p_end = PathPoint {
        anchor: (80.0, 90.0),
        handle_in: Some((70.0, 30.0)),
        handle_out: None,
    };

    let samples = BezierEngine::sample_curve(&p_start, &p_end, 16);
    assert_eq!(samples.len(), 17);
    assert_eq!(samples.first().unwrap().0, 10.0);
    assert_eq!(samples.last().unwrap().0, 80.0);

    let mut buf = PixelBuffer::new(100, 100);
    BezierEngine::rasterize_curve(&mut buf, &p_start, &p_end, Color::rgb(255, 0, 0));
    // Verify some pixel along curve was drawn
    assert_eq!(buf.get_pixel(10, 10).unwrap(), Color::rgb(255, 0, 0));
}
