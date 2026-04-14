use std::path::PathBuf;

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

pub fn ensure_dir(path: &PathBuf) -> PathBuf {
    std::fs::create_dir_all(path).expect(&format!("Failed to create {:?}", path));
    path.to_owned()
}
