use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
