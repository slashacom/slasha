use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let features: Vec<String> = env::vars()
        .filter_map(|(key, _)| {
            key.strip_prefix("CARGO_FEATURE_")
                .map(|feature| feature.to_lowercase().replace('_', "-"))
        })
        .collect();

    println!(
        "cargo:warning=compiling with features: {}",
        if features.is_empty() {
            "none".to_string()
        } else {
            features.join(", ")
        }
    );

    let vars = [
        "PROFILE",
        "TARGET",
        "CARGO_CFG_TARGET_FAMILY",
        "CARGO_CFG_TARGET_OS",
        "CARGO_CFG_TARGET_ARCH",
        "CARGO_CFG_TARGET_POINTER_WIDTH",
        "CARGO_CFG_TARGET_ENDIAN",
        "CARGO_CFG_TARGET_FEATURE",
        "HOST",
    ];

    for var in vars {
        println!(
            "cargo:rustc-env={}={}",
            var,
            env::var(var).unwrap_or_else(|_| "unknown".to_string())
        );
    }

    let build_timestamp = chrono::Local::now()
        .naive_local()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);

    Ok(())
}
