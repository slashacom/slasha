pub enum EnvRef {
    Ref(RefSource, String),
    Literal,
}

pub enum RefSource {
    Own,             // ${{ KEY }} - own scope (app or service)
    Service(String), // ${{ serviceName.KEY }}
    System,          // ${{ SLASHA.KEY }}
}

pub fn parse_env_ref(value: &str) -> EnvRef {
    let inner = value
        .trim()
        .strip_prefix("${{")
        .and_then(|s| s.strip_suffix("}}"))
        .map(str::trim);

    match inner {
        None => EnvRef::Literal,
        Some(s) => match s.split_once('.') {
            Some(("SLASHA", key)) => EnvRef::Ref(RefSource::System, key.trim().to_string()),
            Some((namespace, key)) => EnvRef::Ref(
                RefSource::Service(namespace.trim().to_string()),
                key.trim().to_string(),
            ),
            None => EnvRef::Ref(RefSource::Own, s.trim().to_string()),
        },
    }
}
