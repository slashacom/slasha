use std::collections::{HashMap, HashSet, VecDeque};

use once_cell::sync::Lazy;
use regex::Regex;

use super::{DeploymentError, DeploymentResult};

static ENV_REF_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{\{([^}]*)\}\}").unwrap());

pub enum RefSource {
    Own,
    Service(String),
    System,
}

struct EnvToken {
    source: RefSource,
    key: String,
}

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

fn collect_own_refs(value: &str) -> Vec<&str> {
    let mut refs = Vec::new();

    for cap in ENV_REF_REGEX.captures_iter(value) {
        let inner = cap.get(1).unwrap().as_str().trim();
        if !inner.contains('.') {
            refs.push(inner);
        }
    }

    refs
}

// resolves vars in dependency order so Own refs are always available when
// needed
pub fn topo_sort_vars<V: Clone>(
    vars: Vec<V>,
    key_fn: impl Fn(&V) -> &str,
    value_fn: impl Fn(&V) -> &str,
) -> DeploymentResult<Vec<V>> {
    let var_map: HashMap<&str, &V> = vars.iter().map(|v| (key_fn(v), v)).collect();

    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for var in &vars {
        let mut own_refs = collect_own_refs(value_fn(var));
        own_refs.retain(|dep| var_map.contains_key(dep));

        deps.insert(key_fn(var), own_refs);
    }

    let mut in_degree: HashMap<&str, usize> = vars.iter().map(|v| (key_fn(v), 0)).collect();

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

    let mut sorted: Vec<V> = Vec::with_capacity(vars.len());

    while let Some(key) = queue.pop_front() {
        sorted.push(var_map[key].clone());

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

    if sorted.len() != vars.len() {
        let resolved_keys: HashSet<&str> = sorted.iter().map(&key_fn).collect();
        let cycle_keys: Vec<&str> = vars
            .iter()
            .map(key_fn)
            .filter(|k| !resolved_keys.contains(k))
            .collect();

        return Err(DeploymentError::EnvResolveFailed(format!(
            "Circular dependency detected among env vars: {:?}",
            cycle_keys
        )));
    }

    Ok(sorted)
}

pub fn resolve_env_value(
    value: &str,
    mut resolver: impl FnMut(&RefSource, &str) -> DeploymentResult<String>,
) -> DeploymentResult<String> {
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
