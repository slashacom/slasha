pub fn not_empty(value: &str, _context: &()) -> garde::Result {
    if value.trim().is_empty() {
        return Err(garde::Error::new("is required"));
    }

    Ok(())
}

pub fn valid_service_name(value: &str, _context: &()) -> garde::Result {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(garde::Error::new("is required"));
    }

    if trimmed.len() > 63 {
        return Err(garde::Error::new("must not exceed 63 characters"));
    }

    let is_valid = trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !trimmed.starts_with('-')
        && !trimmed.ends_with('-');

    if !is_valid {
        return Err(garde::Error::new(
            "must consist of lowercase alphanumeric characters or '-' and must start and end with an alphanumeric character",
        ));
    }

    Ok(())
}
