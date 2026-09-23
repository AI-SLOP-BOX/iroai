use iroai_core::brush_dynamics::{PaperTextureMask, ScatterGenerator, WetMediaMixer};
use iroai_core::icc_engine::Clut3D;
use iroai_core::psd_tagged_blocks::PsdTaggedBlockParser;
use iroai_core::scratch_disk::{AsyncTileStreamer, StreamResponse, TileKey};

#[test]
fn test_psd_tagged_blocks_effects_and_text() {
    let mut data = Vec::new();

    // 1. Tagged Block: lrFX (0x6C 0x72 0x46 0x58)
    data.extend_from_slice(b"8BIM");
    data.extend_from_slice(b"lrFX");
    let mut lrfx_payload = Vec::new();
    lrfx_payload.extend_from_slice(&0u16.to_be_bytes()); // version
    lrfx_payload.extend_from_slice(&1u16.to_be_bytes()); // count = 1
    // Drop shadow record: sig (8BIM) + 'dsdw' + len + payload
    lrfx_payload.extend_from_slice(b"8BIM");
    lrfx_payload.extend_from_slice(b"dsdw");
    let mut dsdw_payload = vec![0u8; 30];
    dsdw_payload[7] = 8; // blur = 8
    dsdw_payload[11] = 100; // intensity = 100
    dsdw_payload[15] = 120; // angle = 120
    dsdw_payload[19] = 15; // distance = 15
    dsdw_payload[24] = 1; // enabled = true
    lrfx_payload.extend_from_slice(&(dsdw_payload.len() as u32).to_be_bytes());
    lrfx_payload.extend_from_slice(&dsdw_payload);

    data.extend_from_slice(&(lrfx_payload.len() as u32).to_be_bytes());
    data.extend_from_slice(&lrfx_payload);

    // 2. Tagged Block: SoCo (Solid Color)
    data.extend_from_slice(b"8BIM");
    data.extend_from_slice(b"SoCo");
    let soco_payload = vec![0, 0, 0, 0, 0, 255, 128, 64];
    data.extend_from_slice(&(soco_payload.len() as u32).to_be_bytes());
    data.extend_from_slice(&soco_payload);

    let parsed = PsdTaggedBlockParser::parse(&data);
    assert!(parsed.effects.is_some());
    let fx = parsed.effects.unwrap();
    assert!(fx.drop_shadow.is_some());
    let dsdw = fx.drop_shadow.unwrap();
    assert!(dsdw.enabled);
    assert_eq!(dsdw.blur, 8.0);
    assert_eq!(dsdw.distance, 15.0);

    assert!(parsed.solid_color.is_some());
    let soco = parsed.solid_color.unwrap();
    assert_eq!(soco.r, 255);
    assert_eq!(soco.g, 128);
    assert_eq!(soco.b, 64);
}

#[test]
fn test_clut_3d_trilinear_interpolation() {
    // 2x2x2 RGB cube (grid_points = 2, output_channels = 3)
    // Indices: (0,0,0) -> 0.0, (1,1,1) -> 1.0
    let mut table = vec![0.0; 2 * 2 * 2 * 3];
    for z in 0..2 {
        for y in 0..2 {
            for x in 0..2 {
                let idx = (z * 4 + y * 2 + x) * 3;
                table[idx] = x as f32; // Red depends on x
                table[idx + 1] = y as f32; // Green depends on y
                table[idx + 2] = z as f32; // Blue depends on z
            }
        }
    }

    let clut = Clut3D::new(2, 3, table);

    // Sample exactly in the center: (0.5, 0.5, 0.5)
    let sampled = clut.sample_trilinear(0.5, 0.5, 0.5);
    assert_eq!(sampled.len(), 3);
    assert!((sampled[0] - 0.5).abs() < 1e-4);
    assert!((sampled[1] - 0.5).abs() < 1e-4);
    assert!((sampled[2] - 0.5).abs() < 1e-4);

    // Sample arbitrary point (0.25, 0.75, 1.0)
    let s2 = clut.sample_trilinear(0.25, 0.75, 1.0);
    assert!((s2[0] - 0.25).abs() < 1e-4);
    assert!((s2[1] - 0.75).abs() < 1e-4);
    assert!((s2[2] - 1.0).abs() < 1e-4);
}

#[test]
fn test_async_tile_streamer_background_paging() {
    let temp_dir = std::env::temp_dir();
    let streamer = AsyncTileStreamer::new(&temp_dir, 4).expect("Failed to spawn AsyncTileStreamer");

    let key = TileKey {
        layer_id: 10,
        tx: 5,
        ty: 2,
    };
    let payload = vec![77u8; 16384];

    // Asynchronously evict to disk
    streamer.queue_evict(key, payload.clone()).expect("Evict failed");

    // Wait briefly for worker thread to process
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Request fetch back
    streamer.queue_fetch(key).expect("Fetch failed");

    // Wait and receive
    let mut fetched = None;
    for _ in 0..20 {
        if let Some(StreamResponse::Fetched(k, data)) = streamer.try_recv_response() {
            if k == key {
                fetched = Some(data);
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap()[0], 77);
}

#[test]
fn test_wet_media_and_scatter_brush() {
    // 1. Wet Media color pickup
    let mut wet_brush = WetMediaMixer::new([1.0, 0.0, 0.0, 1.0], 0.5, 0.8); // Red loaded brush, 50% wet
    let canvas_pixel = [0.0, 0.0, 1.0, 1.0]; // Blue on canvas

    let deposited = wet_brush.deposit_and_pickup(canvas_pixel);
    // Deposited color should have blended some blue into the red
    assert!(deposited[0] > 0.0); // Red component
    assert!(deposited[2] > 0.0); // Picked up blue component

    // 2. Scatter Generator
    let mut scatter = ScatterGenerator::new(10.0, 8);
    let particles = scatter.generate_scatter(100.0, 100.0);
    assert_eq!(particles.len(), 8);
    for (px, py) in particles {
        let dist = ((px - 100.0).powi(2) + (py - 100.0).powi(2)).sqrt();
        assert!(dist <= 10.0 + 1e-3);
    }

    // 3. Paper Texture Mask
    let paper = PaperTextureMask::new(16.0, 1.5);
    let d1 = paper.evaluate_density(0.0, 0.0);
    let d2 = paper.evaluate_density(50.0, 25.0);
    assert!((0.0..=1.0).contains(&d1));
    assert!((0.0..=1.0).contains(&d2));
}

#[test]
fn test_bicubic_and_smart_object_fidelity() {
    use iroai_core::buffer::PixelBuffer;
    use iroai_core::color::Color;
    use iroai_core::smart_object::SmartObject;
    use iroai_core::transform::Transform;

    let mut buf = PixelBuffer::new(10, 10);
    buf.set_pixel(5, 5, Color::rgb(200, 100, 50));

    // Bicubic sampling smooth interpolator
    let sampled = Transform::sample_bicubic(&buf, 5.0, 5.0);
    assert_eq!(sampled.r, 200);
    assert_eq!(sampled.g, 100);
    assert_eq!(sampled.b, 50);

    // Smart object non-destructive 2x scaling
    let mut smart = SmartObject::new(buf);
    smart.scale_x = 2.0;
    smart.scale_y = 2.0;
    let rendered = smart.render(20, 20);
    assert_eq!(rendered.width, 20);
    assert_eq!(rendered.height, 20);
}

#[test]
fn test_selection_feather_and_hdr_buffer() {
    use iroai_core::buffer::{PixelBuffer, PixelBuffer16};
    use iroai_core::color::Color;
    use iroai_core::selection::{SelectionMask, SelectionOp};

    // 1. Selection Feather (soft boundary transition)
    let mut mask = SelectionMask::new(20, 20);
    mask.select_rect(5, 5, 10, 10, SelectionOp::New);
    assert_eq!(mask.get_value(5, 5), 255);
    assert_eq!(mask.get_value(0, 0), 0);

    mask.feather(3.0);
    let edge_val = mask.get_value(5, 5);
    // Boundary should soften into a gradual feather gradient
    assert!(edge_val > 0 && edge_val < 255);

    // 2. 16-bit HDR buffer roundtrip without banding
    let mut buf8 = PixelBuffer::new(4, 4);
    buf8.set_pixel(0, 0, Color::rgba(128, 64, 32, 255));
    let buf16 = PixelBuffer16::from_buffer8(&buf8);
    // 128 maps to ~32896 in 16-bit range
    assert!(buf16.data[0] > 32000 && buf16.data[0] < 33500);

    let roundtrip8 = buf16.to_buffer8();
    assert_eq!(roundtrip8.data[0], 128);
    assert_eq!(roundtrip8.data[1], 64);
    assert_eq!(roundtrip8.data[2], 32);
}
