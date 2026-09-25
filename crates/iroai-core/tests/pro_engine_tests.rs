use iroai_core::brush_dynamics::{interpolate_catmull_rom, PressureCurve, StrokeStabilizer, StylusInput};
use iroai_core::color::Color;
use iroai_core::icc_engine::ColorManagementEngine;
use iroai_core::psd_resources::PsdResourceParser;
use iroai_core::scratch_disk::{ScratchDiskManager, TileKey, TILE_BYTES};

#[test]
fn test_psd_image_resource_parser() {
    let mut data = Vec::new();

    // 8BIM Resource 1: ResolutionInfo (0x03ED)
    data.extend_from_slice(b"8BIM");
    data.extend_from_slice(&0x03EDu16.to_be_bytes()); // ID
    data.push(0); // empty Pascal string name
    data.push(0); // padding byte for word alignment
    let res_payload: [u8; 16] = [
        0x01, 0x2C, 0x00, 0x00, // 300.0 DPI (16.16 fixed point: 300 = 0x012C0000)
        0x00, 0x01, // inches
        0x00, 0x01, // width display
        0x01, 0x2C, 0x00, 0x00, // 300.0 DPI
        0x00, 0x01, // inches
        0x00, 0x01, // height display
    ];
    data.extend_from_slice(&(res_payload.len() as u32).to_be_bytes());
    data.extend_from_slice(&res_payload);

    // 8BIM Resource 2: Global Angle (0x043D)
    data.extend_from_slice(b"8BIM");
    data.extend_from_slice(&0x043Du16.to_be_bytes());
    data.push(0);
    data.push(0);
    let angle_payload: [u8; 4] = 120i32.to_be_bytes();
    data.extend_from_slice(&(angle_payload.len() as u32).to_be_bytes());
    data.extend_from_slice(&angle_payload);

    let parsed = PsdResourceParser::parse_resources(&data);
    assert!(parsed.resolution.is_some());
    let res = parsed.resolution.unwrap();
    assert_eq!(res.h_res, 300.0);
    assert_eq!(res.v_res, 300.0);
    assert!(res.is_inches);

    assert_eq!(parsed.global_angle, Some(120));
}

#[test]
fn test_icc_profile_parser_and_cmm_transform() {
    // Generate valid synthetic ICC v2 header with D65 matrix profile
    let mut icc_bytes = vec![0u8; 128 + 4 + 4 * 12 + 64];
    // Signature 'acsp' at offset 36..40
    icc_bytes[36..40].copy_from_slice(b"acsp");
    // Color space 'RGB ' at offset 16..20
    icc_bytes[16..20].copy_from_slice(b"RGB ");
    // Connection space 'XYZ ' at offset 20..24
    icc_bytes[20..24].copy_from_slice(b"XYZ ");

    // Tag count = 1
    icc_bytes[128..132].copy_from_slice(&1u32.to_be_bytes());
    // Tag 1: 'rXYZ'
    let r_xyz_offset = 144u32;
    icc_bytes[132..136].copy_from_slice(b"rXYZ");
    icc_bytes[136..140].copy_from_slice(&r_xyz_offset.to_be_bytes());
    icc_bytes[140..144].copy_from_slice(&20u32.to_be_bytes());

    // 'XYZ ' type tag
    let r_off = r_xyz_offset as usize;
    icc_bytes[r_off..r_off + 4].copy_from_slice(b"XYZ ");
    // X = 0.4360657 -> fixed-point s15Fixed16 = round(0.4360657 * 65536) = 28577
    let x_fix: i32 = (0.4360657f32 * 65536.0) as i32;
    let y_fix: i32 = (0.2224884f32 * 65536.0) as i32;
    let z_fix: i32 = (0.0139160f32 * 65536.0) as i32;
    icc_bytes[r_off + 8..r_off + 12].copy_from_slice(&x_fix.to_be_bytes());
    icc_bytes[r_off + 12..r_off + 16].copy_from_slice(&y_fix.to_be_bytes());
    icc_bytes[r_off + 16..r_off + 20].copy_from_slice(&z_fix.to_be_bytes());

    let profile = ColorManagementEngine::parse_icc(&icc_bytes).expect("ICC parse failed");
    assert_eq!(profile.color_space, "RGB ");
    assert!((profile.red_colorant.x - 0.4360657).abs() < 0.001);

    // Test CMM transformation: Linear sRGB -> PCS XYZ -> Inverted
    let src_color = Color::rgb(200, 100, 50);
    let pcs_xyz = ColorManagementEngine::rgb_to_pcs_xyz(src_color, &profile);
    assert!(pcs_xyz.x > 0.0 && pcs_xyz.y > 0.0);

    let roundtrip_rgb = ColorManagementEngine::pcs_xyz_to_rgb(pcs_xyz, &profile);
    assert!((roundtrip_rgb.r as i32 - src_color.r as i32).abs() <= 1);
    assert!((roundtrip_rgb.g as i32 - src_color.g as i32).abs() <= 1);
    assert!((roundtrip_rgb.b as i32 - src_color.b as i32).abs() <= 1);
}

#[test]
fn test_scratch_disk_lru_paging() {
    let temp_dir = std::env::temp_dir();
    // Capacity = 4 tiles in physical RAM
    let mut manager = ScratchDiskManager::new(&temp_dir, 4).expect("Failed to create scratch manager");

    assert_eq!(manager.ram_tile_count(), 0);
    assert_eq!(manager.disk_tile_count(), 0);

    // Put 6 tiles (Tiles 0..5), exceeding RAM capacity of 4
    for i in 0..6u32 {
        let key = TileKey { layer_id: 1, tx: i, ty: 0 };
        let mut tile_data = vec![i as u8; TILE_BYTES];
        tile_data[0] = 42 + i as u8;
        manager.put_tile(key, tile_data).expect("put_tile failed");
    }

    // Now RAM tile count should be 4, and 2 tiles should have been paged to disk
    assert_eq!(manager.ram_tile_count(), 4);
    assert_eq!(manager.disk_tile_count(), 2);

    // Read back Tile 0 (which was swapped out to disk)
    let key0 = TileKey { layer_id: 1, tx: 0, ty: 0 };
    let retrieved0 = manager.get_tile(key0).expect("get_tile failed").expect("tile 0 missing");
    assert_eq!(retrieved0[0], 42);
    assert_eq!(retrieved0.len(), TILE_BYTES);

    // Verify it was paged back into RAM
    assert_eq!(manager.ram_tile_count(), 4);
}

#[test]
fn test_stylus_dynamics_and_stabilization() {
    // 1. Pressure Curve
    let curve = PressureCurve {
        deadzone_low: 0.1,
        deadzone_high: 0.9,
        gamma: 2.0, // Hard curve
    };
    assert_eq!(curve.evaluate(0.05), 0.0); // Inside deadzone low
    assert_eq!(curve.evaluate(0.95), 1.0); // Inside deadzone high
    let mid_p = curve.evaluate(0.5); // (0.5 - 0.1)/0.8 = 0.5^2 = 0.25
    assert!((mid_p - 0.25).abs() < 0.001);

    // 2. Stroke Stabilizer (Exponential Moving Average)
    let mut stabilizer = StrokeStabilizer::new(0.5);
    let p1 = StylusInput {
        x: 100.0,
        y: 100.0,
        pressure: 0.5,
        tilt_altitude: 1.0,
        tilt_azimuth: 0.0,
        timestamp_ms: 1000,
    };
    let out1 = stabilizer.begin_stroke(p1);
    assert_eq!(out1.x, 100.0);

    let p2 = StylusInput {
        x: 200.0,
        y: 200.0,
        pressure: 0.9,
        tilt_altitude: 1.0,
        tilt_azimuth: 0.0,
        timestamp_ms: 1016,
    };
    let out2 = stabilizer.update(p2);
    // alpha = 0.5 -> 100 + (200 - 100) * 0.5 = 150
    assert_eq!(out2.x, 150.0);
    assert_eq!(out2.y, 150.0);

    // 3. Catmull-Rom Spline Interpolation
    let pts = interpolate_catmull_rom(
        (0.0, 0.0),
        (10.0, 10.0),
        (20.0, 30.0),
        (30.0, 30.0),
        4,
    );
    assert_eq!(pts.len(), 5);
    assert_eq!(pts[0], (10.0, 10.0));
    assert_eq!(pts[4], (20.0, 30.0));
}
