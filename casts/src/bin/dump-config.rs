use anyhow::Result;
use casts::config::ClientConfigBuilder;
use commit_store::config::ServerConfigBuilder;

fn main() -> Result<()> {
    let server_cfg = ServerConfigBuilder::default().build();
    let client_cfg = ClientConfigBuilder::default().build()?;
    println!("server_cfg: {:?}", server_cfg);
    println!("client_cfg: {:?}", client_cfg);
    Ok(())
}
