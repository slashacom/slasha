pub fn http_url(value: &str, _: &()) -> garde::Result {
    let valid = reqwest::Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .unwrap_or(false);

    valid
        .then_some(())
        .ok_or_else(|| garde::Error::new("must be a valid HTTP or HTTPS URL"))
}

pub fn optional_http_url(value: &Option<String>, context: &()) -> garde::Result {
    match value {
        Some(value) => http_url(value, context),
        None => Ok(()),
    }
}

pub fn positive_float(value: &f32, _: &()) -> garde::Result {
    (value.is_finite() && *value > 0.0)
        .then_some(())
        .ok_or_else(|| garde::Error::new("must be greater than zero"))
}
