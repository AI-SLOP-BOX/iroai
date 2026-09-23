use iroai_core::{
    ActionCommand, ActionSequence, BatchJobConfig, BatchProcessor, ExportFormat,
    FilterPlugin, ImageIo, PhotoAdjustments, PixelBuffer, PluginManifest,
    PluginPermission, PluginRegistry,
};
use std::path::PathBuf;

#[test]
fn test_action_resize_and_levels() {
    let mut doc = iroai_core::Document::new(100, 100, "Action Test");
    let mut seq = ActionSequence::new("Scale and Level");
    seq.add_command(ActionCommand::Resize { width: 50, height: 50 });
    seq.add_command(ActionCommand::ApplyLevels { black_point: 10, gamma: 1.2, white_point: 240 });

    assert!(seq.execute_all(&mut doc).is_ok());
    assert_eq!(doc.width, 50);
    assert_eq!(doc.height, 50);
}

#[test]
fn test_action_json_serialization() {
    let mut seq = ActionSequence::new("My Action");
    seq.add_command(ActionCommand::FlipHorizontal);
    seq.add_command(ActionCommand::ApplyGaussianBlur { radius: 4 });

    let json = seq.to_json().expect("Failed to serialize ActionSequence");
    let restored = ActionSequence::from_json(&json).expect("Failed to deserialize ActionSequence");
    assert_eq!(restored.name, "My Action");
    assert_eq!(restored.commands.len(), 2);
}

#[test]
fn test_batch_processor_execution() {
    let temp_dir = std::env::temp_dir().join(format!("iroai_batch_test_{}", uuid::Uuid::new_v4()));
    let in_dir = temp_dir.join("input");
    let out_dir = temp_dir.join("output");
    std::fs::create_dir_all(&in_dir).unwrap();

    let img1 = in_dir.join("test1.png");
    let img2 = in_dir.join("test2.png");
    let doc1 = iroai_core::Document::new(20, 20, "Doc1");
    let doc2 = iroai_core::Document::new(30, 30, "Doc2");
    ImageIo::export_png(&doc1, &img1).unwrap();
    ImageIo::export_png(&doc2, &img2).unwrap();

    let mut actions = ActionSequence::new("Batch Resize");
    actions.add_command(ActionCommand::Resize { width: 16, height: 16 });

    let config = BatchJobConfig {
        input_files: vec![img1, img2],
        output_directory: out_dir.clone(),
        output_format: ExportFormat::Png,
        filename_prefix: "thumb_".to_string(),
        filename_suffix: "".to_string(),
        sequential_numbering: true,
        continue_on_error: true,
        actions,
    };

    let progress = BatchProcessor::process_batch(&config, |_| true);
    assert_eq!(progress.succeeded_count, 2);
    assert_eq!(progress.failed_count, 0);

    // Verify processed output files exist and are resized
    let out_file1 = out_dir.join("thumb_test10001.png");
    let out_file2 = out_dir.join("thumb_test20002.png");
    assert!(out_file1.exists());
    assert!(out_file2.exists());

    let loaded1 = ImageIo::load_image(&out_file1).unwrap();
    assert_eq!(loaded1.width, 16);
    assert_eq!(loaded1.height, 16);

    let _ = std::fs::remove_dir_all(temp_dir);
}

struct TestSepiaPlugin;
impl FilterPlugin for TestSepiaPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "org.iroai.plugin.sepia".to_string(),
            name: "Sepia Tone".to_string(),
            version: "1.0.0".to_string(),
            author: "Iroai Team".to_string(),
            description: "Applies vintage sepia tone".to_string(),
            permissions: vec![PluginPermission::ModifyPixels],
        }
    }

    fn process_pixels(&self, buffer: &mut PixelBuffer, _params: &serde_json::Value) -> Result<(), String> {
        for chunk in buffer.data.chunks_exact_mut(4) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            chunk[0] = (r * 0.393 + g * 0.769 + b * 0.189).min(255.0) as u8;
            chunk[1] = (r * 0.349 + g * 0.686 + b * 0.168).min(255.0) as u8;
            chunk[2] = (r * 0.272 + g * 0.534 + b * 0.131).min(255.0) as u8;
        }
        Ok(())
    }
}

#[test]
fn test_plugin_registry_and_permissions() {
    let mut registry = PluginRegistry::new();
    registry.register_filter(Box::new(TestSepiaPlugin));

    let mut buf = PixelBuffer::new(10, 10);
    buf.fill(iroai_core::Color::rgb(200, 200, 200));

    let res = registry.execute_filter("org.iroai.plugin.sepia", &mut buf, &serde_json::json!({}));
    assert!(res.is_ok());

    let px = buf.get_pixel(0, 0).unwrap();
    assert!(px.r > px.b, "Sepia red should be greater than blue");
}
