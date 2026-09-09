//! What the v1 Cordova app left behind when this build replaces it in Google
//! Play: a SQLCipher database (`databases/encedo_<6 chars>` plus journal) and
//! the secure-key-store file (`files/SKS_KEY_FILEsupersecretto`). v2 cannot
//! read them (the plan chose re-pairing over a one-off migration), so they are
//! removed and the person is told to pair again.

use std::fs;
use std::path::Path;

/// Remove v1 leftovers under the app's data directory. Returns what was
/// removed, empty when there was nothing.
pub fn sweep_v1(app_data_dir: &Path) -> Vec<String> {
    let mut removed = Vec::new();
    // Tauri's app data dir is the app's `files/` dir on Android; v1 wrote next to it.
    let base = app_data_dir.parent().unwrap_or(app_data_dir);
    for dir in [app_data_dir.to_path_buf(), base.join("files"), base.join("databases")] {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_v1 = name.starts_with("SKS_KEY_FILE") || name.starts_with("encedo_");
            if !is_v1 {
                continue;
            }
            let path = entry.path();
            let ok = if path.is_dir() { fs::remove_dir_all(&path).is_ok() } else { fs::remove_file(&path).is_ok() };
            if ok && !removed.contains(&name) {
                removed.push(name);
            }
        }
    }
    removed.sort();
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_v1_files() {
        let root = std::env::temp_dir().join(format!("encedo-legacy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let files = root.join("files");
        let dbs = root.join("databases");
        fs::create_dir_all(&files).unwrap();
        fs::create_dir_all(&dbs).unwrap();
        fs::write(files.join("SKS_KEY_FILEsupersecretto"), b"x").unwrap();
        fs::write(files.join("store.bin"), b"ours").unwrap();
        fs::write(dbs.join("encedo_abc123"), b"db").unwrap();
        fs::write(dbs.join("encedo_abc123-journal"), b"j").unwrap();
        fs::write(dbs.join("other.db"), b"keep").unwrap();
        let removed = sweep_v1(&files);
        assert_eq!(removed, vec!["SKS_KEY_FILEsupersecretto", "encedo_abc123", "encedo_abc123-journal"]);
        assert!(files.join("store.bin").exists());
        assert!(dbs.join("other.db").exists());
        assert!(sweep_v1(&files).is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
