fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    nlprule_build::BinaryBuilder::new(
        &["en"],
        std::env::var("OUT_DIR").expect("OUT_DIR must be set during build"),
    )
    .build()
    .expect("failed to fetch nlprule binaries")
    .validate()
    .expect("downloaded nlprule binaries must be valid");
}
