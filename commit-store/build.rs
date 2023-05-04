fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/proto/commit_store.proto")?;
    Ok(())
}
