use iroai_core::{
    Color, ColorSpace, HdrBuffer, PhotoAdjustments, PhotoProcessor, PixelBuffer,
    RawMetadata, RawDevelopSettings,
};

#[test]
fn test_srgb_linear_roundtrip() {
    let original = Color::rgba(128, 64, 200, 255);
    let (lr, lg, lb, la) = original.to_linear_f32();
    let recon = Color::from_linear_f32(lr, lg, lb, la);
    assert!((original.r as i32 - recon.r as i32).abs() <= 1);
    assert!((original.g as i32 - recon.g as i32).abs() <= 1);
    assert!((original.b as i32 - recon.b as i32).abs() <= 1);
    assert_eq!(original.a, recon.a);
}

#[test]
fn test_hdr_buffer_exposure() {
    let mut buf = PixelBuffer::new(10, 10);
    buf.fill(Color::rgb(64, 64, 64));
    let mut hdr = HdrBuffer::from_pixel_buffer(&buf);
    
    // +1 EV should roughly double linear radiance
    hdr.apply_exposure(1.0);
    let out = hdr.to_pixel_buffer();
    let px = out.get_pixel(0, 0).unwrap();
    // In sRGB, 64 linear doubled should be visibly brighter (> 85)
    assert!(px.r > 80);
}

#[test]
fn test_photo_adjustments_temperature_and_vibrance() {
    let mut buf = PixelBuffer::new(4, 4);
    buf.fill(Color::rgb(100, 100, 100));

    let mut adj = PhotoAdjustments::default();
    adj.temperature = 50.0; // Warm (more red, less blue)
    adj.vibrance = 30.0;
    PhotoProcessor::apply_photo_adjustments(&mut buf, &adj);

    let px = buf.get_pixel(0, 0).unwrap();
    assert!(px.r > px.b, "Red should exceed Blue for warm color temperature");
}

#[test]
fn test_photo_adjustments_tone_curve_and_levels() {
    let mut buf = PixelBuffer::new(4, 4);
    buf.fill(Color::rgb(50, 50, 50));

    PhotoProcessor::apply_levels(&mut buf, 20, 1.5, 230);
    let px = buf.get_pixel(0, 0).unwrap();
    // With gamma 1.5 and bp 20, value 50 should be mapped upwards
    assert!(px.r > 50);
}

#[test]
fn test_gamut_detection() {
    let mut buf = PixelBuffer::new(2, 2);
    buf.set_pixel(0, 0, Color::rgb(255, 128, 64)); // Clamped channel
    buf.set_pixel(1, 1, Color::rgb(100, 100, 100)); // Within gamut

    let warnings = PhotoProcessor::detect_out_of_gamut(&buf, ColorSpace::Srgb);
    assert_eq!(warnings[0], true);
    assert_eq!(warnings[3], false);
}

#[test]
fn test_raw_metadata_and_settings() {
    let meta = RawMetadata::default();
    assert_eq!(meta.camera_make, "Sony");
    let settings = RawDevelopSettings::default();
    assert_eq!(settings.white_balance_temp, 5500.0);
}
