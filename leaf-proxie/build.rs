use std::result::Result;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/proxie.proto")?;
    tonic_build::compile_protos("proto/echo.proto")?;
    Ok(())
}