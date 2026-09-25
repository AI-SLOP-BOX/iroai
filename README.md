# Iroai（色合い）

**Iroai** is a lightweight, high-performance raster image editor and precision image processing engine implemented in Rust.

Targeting macOS, Windows, Linux, iPadOS, and Android tablets, Iroai provides a modular architecture with strict separation between headless core processing, modern GUI (egui/wgpu), CLI automation, and pipeline integration.

## Key Features

- **Decoupled Architecture**:
  - `iroai-core`: Headless pure-Rust image manipulation engine with OpenType/TrueType typography and Bezier vector path engine.
  - `iroai-gui`: Modern cross-platform hardware-accelerated GUI with stylus dynamics, interactive Pen tool, in-canvas text tool, and pro panels.
  - `iroai-cli`: Command-line tool for multi-format conversion, resizing, filters, verification, and batch processing.
  - `moufu-adapter-sdk`: Non-blocking integration with Moufu Integration Hub.
  - `iroai-gpu`: Hardware acceleration primitives.
- **Photoshop-Parity Creative Engine**:
  - **Magic Wand & Color Range Selection**: Contiguous & non-contiguous 4-way flood fill, Euclidean color distance tolerance, and fuzzy soft-mask extraction.
  - **4-Corner Perspective Homography (Free Distort)**: 8x9 Gaussian elimination inverse homography mapping with Catmull-Rom bicubic spline filtering.
  - **Content-Aware Scale (Seam Carving)**: Dynamic programming dual-gradient energy analysis to carve low-energy seams while preserving salient objects.
  - **Expanded Photoshop Blend Modes**: Linear Dodge (Add), Linear Burn, Vivid Light, Pin Light, and Hard Mix.
  - **Layer Styles (FX)**: Drop Shadow, Outer Glow (distance-field falloff), Bevel & Emboss (Sobel normals & directional shading), Stroke, and Color Overlay.
  - **Canvas Measurement & Alignment**: Dynamic ruler-drag guide creation with cyan guide lines, magnetic snapping, and pixel grid zoom inspection.
  - **Layer Groups & Folders**: Hierarchical nesting (`parent_id`) with group pass-through visibility and composite buffer blending.
- **Precision Color Management**:
  - sRGB and Linear RGB (scRGB) f32 floating-point conversions.
  - ColorHdr buffer foundation for 32-bit linear floating point / HDR processing.
  - CMYK color model, GCR/UCR separation, subtractive ink simulation, and Total Area Coverage (TAC) overflow warnings.
- **Non-Destructive Retouching & Photo Adjustments**:
  - Liquify Push / Bloat & Pinch brushes, and Spot Healing Brush with annular IDW texture blending.
  - Exposure (EV), White Balance (Temperature / Tint), Highlights & Shadows, Whites & Blacks, Vibrance, Saturation, Clarity, Vignetting, Sharpening, and Gaussian Blur.
  - Tone Curves & Levels adjustment (Black point, Midtone gamma, White point).
  - 18+ Photoshop Blend Modes.
  - Clipping Masks and 8-bit Layer Masks.
- **Vector Pen & Typography**:
  - Interactive Pen Tool with anchor points, cubic Bezier curve editing, direction handles, and path panel.
  - Vector Path stroke, fill, and conversion to selection masks.
  - High-performance TrueType & OpenType font rasterization (`fontdue`) with letter tracking (`tracking_px`) and line-height multipliers.
- **Advanced Filters**:
  - Gaussian Blur, Motion Blur, Radial Spin Blur, Unsharp Mask, and Sobel edge detection.
- **File Compatibility**:
  - Photoshop Document (PSD) multi-layer roundtrip read/write.
  - PNG, JPEG, WebP, and TIFF high-fidelity import/export.
  - `.iroai` atomic ZIP project format with auto-backup and generation rotation (`RecoveryManager`).
- **Performance & Virtual Memory**:
  - Scratch disk LRU cache for memory-constrained large canvas workflows.
  - Multi-scale Tile Pyramid with bilinear subsampling.
  - PackBits run-length encoding.
- **Automation & Batch Processing**:
  - `ActionSequence`: Recordable and JSON-serializable macro actions.
  - `BatchProcessor`: Sequential batch processing with naming presets and error tolerance.
  - `PluginRegistry`: Permission-isolated extensible filter plugin API.

## Building and Running

### Prerequisites
- [Rust toolchain](https://www.rust-lang.org/) (stable, 2021 edition)

### Run GUI
```bash
cargo run -p iroai-gui
```

### Run CLI
```bash
cargo run -p iroai-cli -- convert input.png output.webp
cargo run -p iroai-cli -- resize input.png output.png 800 600
cargo run -p iroai-cli -- photo input.jpg output.jpg 0.5 25.0
cargo run -p iroai-cli -- batch config.json
cargo run -p iroai-cli -- verify project.iroai
```

### Run Tests
```bash
cargo test --workspace
```

## License

Dual-licensed under either:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Third-Party Licenses & Fonts
- **Noto Sans JP**: Licensed under the SIL Open Font License, Version 1.1 (http://scripts.sil.org/OFL).
- **fontdue**: Dual-licensed under MIT OR Apache-2.0.
