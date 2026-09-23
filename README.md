# Iroai（色合い）

**Iroai** is a lightweight, high-performance raster image editor and precision image processing engine implemented in Rust.

Targeting macOS, Windows, Linux, iPadOS, and Android tablets, Iroai provides a modular architecture with strict separation between headless core processing, modern GUI (egui/wgpu), CLI automation, and pipeline integration.

## Key Features

- **Decoupled Architecture**:
  - `iroai-core`: Headless pure-Rust image manipulation engine.
  - `iroai-gui`: Modern cross-platform hardware-accelerated GUI with multi-touch and stylus pressure support.
  - `iroai-cli`: Command-line tool for conversions, resizing, photo filters, and batch processing.
  - `iroai-moufu`: Integration protocol client for the Moufu creative ecosystem.
  - `iroai-gpu`: Hardware acceleration primitives.
- **Precision Color Management**:
  - sRGB and Linear RGB (scRGB) f32 floating-point conversions.
  - ColorHdr buffer foundation for 32-bit linear floating point / HDR processing.
  - CMYK color model, GCR/UCR separation, and Total Area Coverage (TAC) overflow warnings.
- **Non-Destructive Retouching & Photo Adjustments**:
  - Exposure (EV), White Balance (Temperature / Tint), Highlights & Shadows, Whites & Blacks, Vibrance, Saturation, Clarity, Vignetting, Sharpening, and Gaussian Blur.
  - Tone Curves & Levels adjustment (Black point, Midtone gamma, White point).
  - 13 Blend Modes (Normal, Multiply, Screen, Overlay, Color Dodge, Burn, etc.).
  - Non-destructive Layer Styles (Drop Shadow, Stroke, Color Overlay).
  - Clipping Masks and Layer Masks.
- **File Compatibility**:
  - Photoshop Document (PSD) multi-layer roundtrip read/write.
  - PNG, JPEG, WebP, and TIFF high-fidelity import/export.
  - `.iroai` atomic ZIP project format with auto-backup and generation rotation (`RecoveryManager`).
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
