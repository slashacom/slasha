use std::path::{Path, PathBuf};

pub fn ensure_dir(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    std::fs::create_dir_all(path).unwrap_or_else(|_| panic!("Failed to create {:?}", path));
    path.to_path_buf()
}
