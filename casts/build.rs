// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/proto/casts.proto")?;
    Ok(())
}
