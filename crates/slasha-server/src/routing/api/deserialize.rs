use serde::{Deserialize, Deserializer};

pub fn trim_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(|value| value.trim().to_string())
}

pub fn trim_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
        .map(|value| value.map(|value| value.trim().to_string()))
}

pub fn trim_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<String>::deserialize(deserializer).map(|values| {
        values
            .into_iter()
            .map(|value| value.trim().to_string())
            .collect()
    })
}
