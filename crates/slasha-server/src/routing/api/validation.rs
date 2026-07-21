pub fn not_empty(value: &str, _context: &()) -> garde::Result {
    if value.trim().is_empty() {
        return Err(garde::Error::new("is required"));
    }

    Ok(())
}

pub fn normalize_root_dir(value: &str) -> Result<String, String> {
    let value = value.trim();

    if value.contains('\\') {
        return Err("must use forward slashes".to_string());
    }

    if value.contains('\0') {
        return Err("must not contain null bytes".to_string());
    }

    if value.starts_with('/') {
        return Err("must be a path relative to the repository root".to_string());
    }

    let mut parts = Vec::new();

    for part in value.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }

        if part == ".." {
            return Err("must not traverse outside the repository".to_string());
        }

        parts.push(part);
    }

    Ok(parts.join("/"))
}

pub fn valid_root_dir(value: &str, _context: &()) -> garde::Result {
    if let Err(message) = normalize_root_dir(value) {
        return Err(garde::Error::new(message));
    }

    Ok(())
}
