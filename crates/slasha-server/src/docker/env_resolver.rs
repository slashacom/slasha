use std::collections::{HashMap, HashSet, VecDeque};

use once_cell::sync::Lazy;
use regex::Regex;

use super::{DockerError, DockerResult};

/// Regular expression matching environment variable reference expressions formatted as `${{ ... }}`.
static ENV_REF_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{\{([^}]*)\}\}").unwrap());

/// Indicates the origin domain for an environment variable reference token.
pub enum RefSource {
    Own,
    Service(String),
    System,
}

/// Represents a parsed environment variable reference expression containing its origin source and key.
struct EnvToken {
    source: RefSource,
    key: String,
}

/// Parses a raw template reference string into an [`EnvToken`].
///
/// # Arguments
///
/// * `s` - Raw template reference string inner content.
///
/// # Returns
///
/// An [`EnvToken`] categorizing the reference source ([`RefSource`]) and key name.
fn parse_single_ref(s: &str) -> EnvToken {
    match s.split_once('.') {
        Some(("SLASHA", key)) => EnvToken {
            source: RefSource::System,
            key: key.trim().to_string(),
        },
        Some((namespace, key)) => EnvToken {
            source: RefSource::Service(namespace.trim().to_string()),
            key: key.trim().to_string(),
        },
        None => EnvToken {
            source: RefSource::Own,
            key: s.trim().to_string(),
        },
    }
}

/// Extracts all own-variable dependency keys referenced within an environment variable value string.
///
/// # Arguments
///
/// * `value` - Variable value template string to inspect for `${{ VAR }}` references.
///
/// # Returns
///
/// A vector of referenced variable key name strings within the same application context.
fn collect_env_dependencies(value: &str) -> Vec<&str> {
    let mut refs = Vec::new();

    for cap in ENV_REF_REGEX.captures_iter(value) {
        let inner = cap.get(1).unwrap().as_str().trim();

        // A dot indicates a namespaced reference (e.g. service.KEY or SLASHA.KEY)
        if !inner.contains('.') {
            refs.push(inner);
        }
    }

    refs
}

/// Topologically sorts environment variables to resolve inter-variable dependencies without cycles.
///
/// # Arguments
///
/// * `raw_vars` - Map of key-value environment variable strings.
///
/// # Returns
///
/// A [`DockerResult`] containing a vector of `(key, value)` tuples ordered topologically.
pub fn topo_sort_env(raw_vars: &HashMap<String, String>) -> DockerResult<Vec<(&str, &str)>> {
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for (key, value) in raw_vars {
        let mut own_refs = collect_env_dependencies(value);
        own_refs.retain(|dep| raw_vars.contains_key(*dep));
        deps.insert(key.as_str(), own_refs);
    }

    let mut in_degree: HashMap<&str, usize> = raw_vars.keys().map(|k| (k.as_str(), 0)).collect();

    let mut reverse_deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for (key, key_deps) in &deps {
        for dep in key_deps {
            *in_degree.entry(key).or_default() += 1;
            reverse_deps.entry(dep).or_default().push(key);
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|&(_, deg)| deg == &0)
        .map(|(&key, _)| key)
        .collect();

    let mut sorted: Vec<(&str, &str)> = Vec::with_capacity(raw_vars.len());

    while let Some(key) = queue.pop_front() {
        if let Some(val) = raw_vars.get(key) {
            sorted.push((key, val.as_str()));
        }

        if let Some(dependents) = reverse_deps.get(key) {
            for dep in dependents {
                let degree = in_degree.get_mut(dep).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(dep);
                }
            }
        }
    }

    if sorted.len() != raw_vars.len() {
        let resolved_keys: HashSet<&str> = sorted.iter().map(|(k, _)| *k).collect();
        let cycle_keys: Vec<&str> = raw_vars
            .keys()
            .map(|k| k.as_str())
            .filter(|k| !resolved_keys.contains(k))
            .collect();

        return Err(DockerError::EnvResolveFailed(format!(
            "Circular dependency detected among env vars: {:?}",
            cycle_keys
        )));
    }

    Ok(sorted)
}

/// Interpolates template variables in an environment variable string value using a resolver closure.
///
/// # Arguments
///
/// * `value` - Raw variable value string containing potential template expressions.
/// * `resolver` - Closure resolving reference token source ([`RefSource`]) and key to a value.
///
/// # Returns
///
/// A [`DockerResult`] containing the resolved value string.
pub fn resolve_env_value(
    value: &str,
    mut resolver: impl FnMut(&RefSource, &str) -> DockerResult<String>,
) -> DockerResult<String> {
    let mut result = String::with_capacity(value.len());
    let mut last = 0;

    for cap in ENV_REF_REGEX.captures_iter(value) {
        let full = cap.get(0).unwrap();
        let inner = cap[1].trim();

        result.push_str(&value[last..full.start()]);
        let token = parse_single_ref(inner);
        result.push_str(&resolver(&token.source, &token.key)?);
        last = full.end();
    }

    result.push_str(&value[last..]);
    Ok(result)
}
