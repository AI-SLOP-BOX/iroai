use iroai_core::{BackupConfig, Document, RecoveryManager};

#[test]
fn test_recovery_auto_save_and_pruning() {
    let temp_dir = std::env::temp_dir().join(format!("iroai_recov_{}", uuid::Uuid::new_v4()));
    let config = BackupConfig {
        auto_save_interval_secs: 0, // trigger immediately
        max_generations: 2,
        backup_dir: temp_dir.clone(),
    };

    let mut manager = RecoveryManager::new(config);
    let doc = Document::new(16, 16, "MyProject");

    // 1st backup
    let b1 = manager.create_auto_backup(&doc).expect("Backup 1 failed");
    assert!(b1.exists());

    // 2nd backup
    std::thread::sleep(std::time::Duration::from_millis(50));
    let b2 = manager.create_auto_backup(&doc).expect("Backup 2 failed");
    assert!(b2.exists());

    // 3rd backup (should prune oldest to keep max_generations = 2)
    std::thread::sleep(std::time::Duration::from_millis(50));
    let b3 = manager.create_auto_backup(&doc).expect("Backup 3 failed");
    assert!(b3.exists());

    let available = manager.find_recoverable_backups("MyProject");
    assert_eq!(available.len(), 2, "Should keep exactly 2 generations of backups");

    let _ = std::fs::remove_dir_all(temp_dir);
}
