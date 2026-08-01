/// Parses the exposed port from `EXPOSE` instructions in a `Dockerfile`.
///
/// # Arguments
///
/// * `dockerfile_content` - Raw string content of the Dockerfile.
///
/// # Returns
///
/// Option containing the port number (`u16`).
pub fn parse_expose(dockerfile_content: &str) -> Option<u16> {
    for line in dockerfile_content.lines() {
        let trimmed = line.trim();
        if trimmed.to_uppercase().starts_with("EXPOSE ") {
            let rest = trimmed["EXPOSE ".len()..].trim();
            let port_str = rest.split('/').next().unwrap_or("").trim();
            if let Ok(port) = port_str.parse::<u16>() {
                return Some(port);
            }
        }
    }
    None
}

/// Extracts target volume mount paths declared in `VOLUME` instructions of a `Dockerfile`.
///
/// # Arguments
///
/// * `dockerfile_content` - Raw string content of the Dockerfile.
///
/// # Returns
///
/// A vector of container path strings.
pub fn parse_volumes(dockerfile_content: &str) -> Vec<String> {
    let mut current_stage: Vec<String> = Vec::new();
    for raw in dockerfile_content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let upper = line.to_uppercase();
        if upper.starts_with("FROM ") {
            current_stage.clear();
            continue;
        }
        if !upper.starts_with("VOLUME") {
            continue;
        }
        let rest = line["VOLUME".len()..].trim_start();
        let paths = if rest.starts_with('[') {
            parse_volume_exec_form(rest)
        } else {
            parse_volume_shell_form(rest)
        };
        for p in paths {
            let p = p.trim().to_string();
            if !p.is_empty() && !current_stage.contains(&p) {
                current_stage.push(p);
            }
        }
    }
    current_stage
}

fn parse_volume_exec_form(s: &str) -> Vec<String> {
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|part| {
            part.trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_string()
        })
        .filter(|p| !p.is_empty())
        .collect()
}

fn parse_volume_shell_form(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}
