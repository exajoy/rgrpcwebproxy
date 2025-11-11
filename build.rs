fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_FEATURE_TEST_SUPPORT").is_ok() {
        // eprintln!("\x1b[32m[INFO]\x1b[0m building proto files...");
        // println!("cargo:warning=Building test proto file...");
        tonic_prost_build::configure()
            .file_descriptor_set_path("helloworld_descriptor.bin")
            .build_client(true)
            .build_server(true)
            .compile_protos(
                &["src/test_support/proto/helloworld/helloworld.proto"],
                &["src/test_support/proto"],
            )?;
    }
    Ok(())
}
