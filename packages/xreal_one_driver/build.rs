use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_dir = fs::canonicalize(manifest_dir.join("proto"))?;

    let protos_abs = glob::glob(proto_dir.join("**/*.proto").to_str().unwrap())?
        .collect::<Result<Vec<_>, glob::GlobError>>()?;

    protobuf_codegen::Codegen::new()
        .includes(&[proto_dir.to_str().unwrap()])
        .inputs(&protos_abs)
        .cargo_out_dir("protos")
        .run_from_script();

    // Rebuild on any .proto change
    for p in &protos_abs {
        println!("cargo:rerun-if-changed={}", p.display());
    }

    Ok(())
}
