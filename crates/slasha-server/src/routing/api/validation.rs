pub fn not_empty(value: &str, _context: &()) -> garde::Result {
    if value.trim().is_empty() {
        return Err(garde::Error::new("is required"));
    }

    Ok(())
}
