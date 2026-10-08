use todo_core::*;
#[test]
fn failed_atomic_export_preserves_existing_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("backup.json");
    std::fs::write(&path, "previous backup").unwrap();
    let result = atomic_write_with(&path, |file| {
        use std::io::Write;
        file.write_all(b"partial new backup")?;
        Err(std::io::Error::other("simulated disk full"))
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "previous backup");
    atomic_write_with(&path, |file| {
        use std::io::Write;
        file.write_all(b"complete new backup")
    })
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "complete new backup"
    );
}
