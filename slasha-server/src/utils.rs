use std::path::{Path, PathBuf};

pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn ensure_dir(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    std::fs::create_dir_all(path).unwrap_or_else(|_| panic!("Failed to create {:?}", path));
    path.to_path_buf()
}
