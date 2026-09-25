use iroai_core::{
    Color, Document, Filters, Layer, PathPoint, PixelBuffer, SubPath, TextRenderer, VectorPath,
};

#[test]
fn test_typography_engine_rendering_and_measurement() {
    let mut buffer = PixelBuffer::new(300, 100);
    let text = "色合 Iroai Pro 日本語";
    let (w, h) = TextRenderer::measure_text(text, 24.0);
    assert!(w > 0 && h > 0);

    TextRenderer::render_text(&mut buffer, text, 10, 10, 24.0, Color::rgb(255, 0, 0));

    // Verify rendered pixels exist with Red color
    let mut painted_pixels = 0;
    for y in 0..buffer.height {
        for x in 0..buffer.width {
            if let Some(col) = buffer.get_pixel(x, y) {
                if col.r > 200 {
                    painted_pixels += 1;
                }
            }
        }
    }
    assert!(painted_pixels > 50, "OpenType vector text rasterization must paint characters onto buffer");
}

#[test]
fn test_vector_path_bezier_stroke_and_fill() {
    let mut buffer = PixelBuffer::new(100, 100);
    let mut path = VectorPath::new("Test Shape");

    // Triangle path with Bezier curve
    let sub = SubPath {
        points: vec![
            PathPoint {
                anchor: (20.0, 20.0),
                handle_in: None,
                handle_out: Some((40.0, 10.0)),
            },
            PathPoint {
                anchor: (80.0, 20.0),
                handle_in: Some((60.0, 10.0)),
                handle_out: None,
            },
            PathPoint {
                anchor: (50.0, 80.0),
                handle_in: None,
                handle_out: None,
            },
        ],
        closed: true,
    };
    path.subpaths.push(sub);

    // Test Bezier stroke
    path.rasterize_stroke(&mut buffer, Color::rgb(0, 0, 255), 2.0);
    let mut blue_pixels = 0;
    for y in 0..buffer.height {
        for x in 0..buffer.width {
            if let Some(col) = buffer.get_pixel(x, y) {
                if col.b == 255 && col.r == 0 {
                    blue_pixels += 1;
                }
            }
        }
    }
    assert!(blue_pixels > 0, "Stroke path should rasterize Bezier lines");

    // Test vector fill
    let mut fill_buffer = PixelBuffer::new(100, 100);
    path.rasterize_fill(&mut fill_buffer, Color::rgb(0, 255, 0));
    let mut green_pixels = 0;
    for y in 0..fill_buffer.height {
        for x in 0..fill_buffer.width {
            if let Some(col) = fill_buffer.get_pixel(x, y) {
                if col.g == 255 && col.r == 0 {
                    green_pixels += 1;
                }
            }
        }
    }
    assert!(green_pixels > 100, "Fill path should fill inside the closed vector boundary");
}

#[test]
fn test_advanced_filters_blur_unsharp_sobel() {
    let mut buffer = PixelBuffer::new(64, 64);
    buffer.fill(Color::WHITE);

    // Draw a black square in center
    for y in 20..44 {
        for x in 20..44 {
            buffer.set_pixel(x, y, Color::BLACK);
        }
    }

    // 1. Motion blur
    let mut mb_buf = buffer.clone();
    Filters::apply_motion_blur(&mut mb_buf, 0.0, 5);
    // Boundary pixel at x=18, y=30 should become grayish instead of pure white
    let p = mb_buf.get_pixel(18, 30).unwrap();
    assert!(p.r < 255, "Motion blur should smear black square horizontally");

    // 2. Unsharp mask
    let mut unsharp_buf = buffer.clone();
    Filters::apply_unsharp_mask(&mut unsharp_buf, 2, 2.0, 1);
    assert_eq!(unsharp_buf.width, 64);

    // 3. Sobel edge detection
    let mut sobel_buf = buffer.clone();
    Filters::apply_sobel_edge(&mut sobel_buf);
    // Edge at x=20, y=30 should have high intensity
    let edge_pixel = sobel_buf.get_pixel(20, 30).unwrap();
    assert!(edge_pixel.r > 50, "Sobel filter must detect border contrast");
}

#[test]
fn test_non_destructive_text_layer() {
    let mut doc = iroai_core::Document::new(200, 100, "Text Test");
    let text_layer = iroai_core::Layer::new_text(
        "Headline",
        "色合",
        24.0,
        Color::BLACK,
        10,
        10,
        200,
        100,
    );
    let id = text_layer.id;
    doc.layers.push(text_layer);
    doc.active_layer_id = Some(id);

    // Verify initial text layer
    let initial_rendered = doc.layers.last().unwrap().buffer.data.iter().any(|&b| b > 0);
    assert!(initial_rendered, "Text should be rendered initially");

    // Non-destructive update: change text, font size, and color
    let red = Color::rgb(255, 0, 0);
    let layer = doc.active_layer_mut().unwrap();
    layer.update_text("色合 Pro", 32.0, red);

    if let iroai_core::LayerKind::Text { ref text, font_size, color, .. } = layer.kind {
        assert_eq!(text, "色合 Pro");
        assert_eq!(font_size, 32.0);
        assert_eq!(color, red);
    } else {
        panic!("Layer kind should remain Text");
    }

    // Verify red channel has non-zero pixels
    let has_red = layer.buffer.data.chunks_exact(4).any(|c| c[0] > 0 && c[3] > 0);
    assert!(has_red, "Updated text layer should render red pixels");
}

#[test]
fn test_liquify_push_and_bloat() {
    let mut buf = PixelBuffer::new(64, 64);
    // Draw a small 10x10 white square at (27, 27)
    for y in 27..37 {
        for x in 27..37 {
            buf.set_pixel(x, y, Color::WHITE);
        }
    }

    // Liquify Push: push center (32, 32) to the right by dx = 10.0
    let mut pushed_buf = buf.clone();
    iroai_core::Transform::apply_liquify_push(&mut pushed_buf, 32.0, 32.0, 10.0, 0.0, 15.0, 1.0);
    
    // Pixel to the right of original white box (e.g. x=40, y=32) should receive warped color
    let warped_px = pushed_buf.get_pixel(40, 32).unwrap();
    assert!(warped_px.a > 0 || warped_px.r > 0, "Forward warp should shift pixel data towards the right");

    // Liquify Bloat: bloat center (32, 32)
    let mut bloated_buf = buf.clone();
    iroai_core::Transform::apply_liquify_bloat(&mut bloated_buf, 32.0, 32.0, 20.0, 0.8);
    // The white core should deform and expand
    let count_non_zero = bloated_buf.data.chunks_exact(4).filter(|c| c[3] > 0).count();
    assert!(count_non_zero > 0, "Bloat deformation should preserve/distort pixels");
}

#[test]
fn test_cmyk_plates_and_subtractive_compositing() {
    let mut buf = PixelBuffer::new(10, 10);
    // Pure Cyan in RGB is (0, 255, 255)
    buf.fill(Color::rgb(0, 255, 255));

    // Extract Cyan plate: cyan should be dense (low value in white-film density, i.e. 255 - 255 = 0)
    let c_plate = iroai_core::CmykManager::extract_plate(&buf, iroai_core::CmykPlate::Cyan);
    let px = c_plate.get_pixel(5, 5).unwrap();
    assert!(px.r < 10, "Cyan plate density for pure cyan should be fully dark (ink presence)");

    // Extract Magenta plate: should have no magenta ink (density = 255, film white)
    let m_plate = iroai_core::CmykManager::extract_plate(&buf, iroai_core::CmykPlate::Magenta);
    let px_m = m_plate.get_pixel(5, 5).unwrap();
    assert!(px_m.r > 240, "Magenta plate density for pure cyan should be empty / clear");

    // Extract Cyan plate with 50% opacity: density should be halved (around 128)
    let mut half_buf = PixelBuffer::new(10, 10);
    half_buf.fill(Color::rgba(0, 255, 255, 128));
    let c_half_plate = iroai_core::CmykManager::extract_plate(&half_buf, iroai_core::CmykPlate::Cyan);
    let px_half = c_half_plate.get_pixel(5, 5).unwrap();
    assert!(px_half.r >= 120 && px_half.r <= 135, "Cyan plate with 50% alpha should yield ~50% film density (~128), got {}", px_half.r);

    // Test subtractive ink blending
    let base_cmyk = iroai_core::ColorCmyk::new(0.5, 0.0, 0.0, 0.0);
    let ink_overlay = iroai_core::ColorCmyk::new(0.0, 0.6, 0.0, 0.0);
    let blended = iroai_core::CmykManager::blend_cmyk_subtractive(base_cmyk, ink_overlay, 1.0);
    assert!((blended.c - 0.5).abs() < 1e-3);
    assert!((blended.m - 0.6).abs() < 1e-3);
}

#[test]
fn test_spot_healing_annular_texture_blend() {
    let mut buf = PixelBuffer::new(64, 64);
    // Fill background with warm skin tone: (220, 180, 150)
    buf.fill(Color::rgb(220, 180, 150));

    // Create a dark blemish / spot at center (32, 32)
    for y in 30..=34 {
        for x in 30..=34 {
            buf.set_pixel(x, y, Color::rgb(40, 20, 10));
        }
    }
    let spot_before = buf.get_pixel(32, 32).unwrap();
    assert_eq!(spot_before.r, 40, "Spot should be dark before healing");

    // Apply Spot Healing with radius 8.0
    iroai_core::Filters::apply_spot_heal(&mut buf, 32.0, 32.0, 8.0);

    let spot_after = buf.get_pixel(32, 32).unwrap();
    // Center pixel should be harmonized towards surrounding skin tone (~220, ~180, ~150)
    assert!(spot_after.r > 160, "Spot should be repaired towards surrounding skin tone, got {}", spot_after.r);
    assert!(spot_after.g > 130, "Spot should be repaired towards surrounding skin tone, got {}", spot_after.g);
}

#[test]
fn test_non_destructive_adjustment_layers() {
    let mut doc = Document::new(32, 32, "Adj Test");
    // Base layer with mid-gray (100, 100, 100)
    if let Some(base) = doc.active_layer_mut() {
        base.buffer.fill(Color::rgb(100, 100, 100));
    }

    // Add Levels adjustment layer: black_point = 50, gamma = 1.0, white_point = 200
    let adj_layer = Layer::new_adjustment(
        "Levels Adj",
        iroai_core::AdjustmentKind::Levels { black_point: 50, gamma: 1.0, white_point: 200 },
        32,
        32,
    );
    doc.layers.push(adj_layer);

    // Render composite
    let comp = doc.composite();
    let px = comp.get_pixel(16, 16).unwrap();
    // 100 mapped from [50, 200]: (100 - 50)/(200 - 50) = 50/150 = 1/3 => ~85
    assert!(px.r >= 80 && px.r <= 90, "Levels mapped mid-gray correctly, got {}", px.r);

    // Verify non-destructive nature: original base layer still has 100, 100, 100!
    let base_px = doc.layers[0].buffer.get_pixel(16, 16).unwrap();
    assert_eq!(base_px.r, 100, "Base layer pixels remain completely untouched/non-destructive");

    // Modify the adjustment layer live to Exposure +1.0 EV (doubles brightness)
    doc.layers[1].kind = iroai_core::LayerKind::Adjustment(iroai_core::AdjustmentKind::Exposure { ev: 1.0 });
    let comp2 = doc.composite();
    let px2 = comp2.get_pixel(16, 16).unwrap();
    assert_eq!(px2.r, 200, "Live update to Exposure +1 EV doubles 100 -> 200");
}

#[test]
fn test_magic_wand_contiguous_and_color_range() {
    let mut buf = PixelBuffer::new(64, 64);
    buf.fill(Color::WHITE);

    // Draw two separate red squares: (10, 10..20, 20) and (40, 40..50, 50)
    for y in 10..20 {
        for x in 10..20 {
            buf.set_pixel(x, y, Color::rgb(255, 0, 0));
        }
    }
    for y in 40..50 {
        for x in 40..50 {
            buf.set_pixel(x, y, Color::rgb(250, 5, 5));
        }
    }

    // 1. Contiguous Magic Wand at (15, 15) with tolerance 15.0
    let mut mask = iroai_core::SelectionMask::new(64, 64);
    mask.select_magic_wand(&buf, 15, 15, 15.0, true, iroai_core::SelectionOp::New);

    // First square must be selected
    assert_eq!(mask.get_value(15, 15), 255);
    // Background must NOT be selected
    assert_eq!(mask.get_value(0, 0), 0);
    // Second detached red square must NOT be selected because contiguous = true
    assert_eq!(mask.get_value(45, 45), 0);

    // 2. Non-contiguous Magic Wand: both red squares should be selected globally
    let mut global_mask = iroai_core::SelectionMask::new(64, 64);
    global_mask.select_magic_wand(&buf, 15, 15, 15.0, false, iroai_core::SelectionOp::New);
    assert_eq!(global_mask.get_value(15, 15), 255);
    assert_eq!(global_mask.get_value(45, 45), 255);
    assert_eq!(global_mask.get_value(0, 0), 0);

    // 3. Color Range fuzzy selection for Red
    let mut fuzzy_mask = iroai_core::SelectionMask::new(64, 64);
    fuzzy_mask.select_color_range(&buf, Color::rgb(255, 0, 0), 30.0, iroai_core::SelectionOp::New);
    assert_eq!(fuzzy_mask.get_value(15, 15), 255);
    assert!(fuzzy_mask.get_value(45, 45) > 200, "Slightly offset red should have high fuzzy selection intensity");
    assert_eq!(fuzzy_mask.get_value(0, 0), 0);
}

#[test]
fn test_perspective_homography_quad_transform() {
    let mut src = PixelBuffer::new(32, 32);
    src.fill(Color::rgb(0, 200, 100)); // Green square

    // Map the 32x32 square to a perspective trapezoid (narrow top, wide bottom)
    // Quad corners: [TL, TR, BR, BL]
    let quad = [
        (10.0, 5.0),  // TL
        (22.0, 5.0),  // TR
        (30.0, 28.0), // BR
        (2.0, 28.0),  // BL
    ];

    let dst = iroai_core::Transform::transform_perspective_quad(&src, 32, 32, quad);

    // Outside the trapezoid: should be transparent
    assert_eq!(dst.get_pixel(0, 0).unwrap().a, 0);
    assert_eq!(dst.get_pixel(31, 0).unwrap().a, 0);

    // Center of the trapezoid: should be mapped with source green color
    let center_px = dst.get_pixel(16, 16).unwrap();
    assert_eq!(center_px.g, 200);
    assert!(center_px.a > 200);
}

#[test]
fn test_extended_photoshop_blend_modes() {
    let base = Color::rgb(100, 150, 200);
    let src = Color::rgb(80, 120, 160);

    // Linear Burn: max(0, b + s - 255)
    // R: max(0, 100 + 80 - 255) = 0
    // G: max(0, 150 + 120 - 255) = 15
    // B: max(0, 200 + 160 - 255) = 105
    let lb = iroai_core::BlendMode::LinearBurn.blend_pixel(base, src, 1.0);
    assert_eq!(lb.r, 0);
    assert_eq!(lb.g, 15);
    assert_eq!(lb.b, 105);

    // Vivid Light, Pin Light, Hard Mix
    let hm = iroai_core::BlendMode::HardMix.blend_pixel(base, src, 1.0);
    // Hard Mix always outputs extreme posterized 0 or 255 channels
    assert!(hm.r == 0 || hm.r == 255);
    assert!(hm.g == 0 || hm.g == 255);
    assert!(hm.b == 0 || hm.b == 255);
}

#[test]
fn test_layer_styles_outer_glow_and_bevel() {
    let mut buf = PixelBuffer::new(40, 40);
    // Draw an opaque circle in center (20, 20) with radius 8
    for y in 0..40 {
        for x in 0..40 {
            let dx = (x as i32 - 20) as f32;
            let dy = (y as i32 - 20) as f32;
            if dx * dx + dy * dy <= 64.0 {
                buf.set_pixel(x, y, Color::rgb(180, 50, 50));
            }
        }
    }

    let mut style = iroai_core::style::LayerStyle::default();
    style.outer_glow = Some(iroai_core::style::OuterGlow {
        enabled: true,
        color: Color::rgb(255, 255, 0), // Yellow glow
        radius: 6,
        opacity: 1.0,
    });
    style.bevel_emboss = Some(iroai_core::style::BevelAndEmboss {
        enabled: true,
        depth: 2.0,
        size: 3,
        angle_deg: 135.0,
        highlight_opacity: 0.8,
        shadow_opacity: 0.8,
    });

    let rendered = style.render_styled(&buf);

    // 1. Check outer glow exists outside circle radius (e.g. at distance ~11)
    let outside_px = rendered.get_pixel(31, 20).unwrap();
    assert!(outside_px.a > 0, "Outer glow should render outside original shape");
    assert!(outside_px.r > 100 && outside_px.g > 100, "Outer glow color should be yellow tint");

    // 2. Check Bevel highlight and shadow within the circle
    let highlight_px = rendered.get_pixel(16, 16).unwrap();
    let shadow_px = rendered.get_pixel(24, 24).unwrap();
    assert!(highlight_px.r > shadow_px.r, "Bevel highlight should be brighter than shadow on opposing edge");
}

#[test]
fn test_content_aware_seam_carving() {
    let mut buf = PixelBuffer::new(50, 30);
    // Fill background with uniform blue (low energy)
    buf.fill(Color::rgb(30, 80, 200));

    // Place high-contrast vertical feature (high energy) in middle columns (24..26)
    for y in 0..30 {
        for x in 24..=26 {
            buf.set_pixel(x, y, Color::rgb(255, 255, 255));
        }
    }

    // Carve width down from 50 to 40
    let carved = iroai_core::Filters::seam_carve_resize(&buf, 40, 30);
    assert_eq!(carved.width, 40);
    assert_eq!(carved.height, 30);

    // The high-energy white feature should be preserved
    let mut white_pixel_count = 0;
    for y in 0..30 {
        for x in 0..40 {
            let px = carved.get_pixel(x, y).unwrap();
            if px.r > 200 && px.g > 200 && px.b > 200 {
                white_pixel_count += 1;
            }
        }
    }
    assert!(white_pixel_count >= 30 * 2, "Seam carving must preserve salient high-energy white feature");
}
