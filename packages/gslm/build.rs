use std::{env, fs, path::Path};

fn main() {
    napi_build::setup();

    // Surface the npm package version to Rust so `version()` matches what
    // users installed. Cargo crate versions stay 0.0.0 (crates are unpublished).
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let pkg_json = Path::new(&manifest_dir).join("package.json");
    println!("cargo:rerun-if-changed={}", pkg_json.display());
    let raw = fs::read_to_string(&pkg_json).expect("read package.json");
    let version = raw
        .lines()
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("\"version\":")
                .map(|v| v.trim().trim_end_matches(',').trim_matches('"').to_string())
        })
        .expect("package.json must contain a top-level \"version\"");
    println!("cargo:rustc-env=GSLM_PACKAGE_VERSION={version}");
}
