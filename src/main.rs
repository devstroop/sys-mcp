use sys_mcp::config::ServerConfig;
use sys_mcp::gui::backend::local::LocalBackend;
use sys_mcp::gui::GuiClient;
use sys_mcp::mcp::server::GuiMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig::from_args();

    // Initialize logging
    if config.debug {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    log::info!("gui-mcp v{} starting...", env!("CARGO_PKG_VERSION"));

    // Create the local GUI backend
    let backend = LocalBackend::new(config.debug)?;
    let client = GuiClient::new(backend);

    // Create and run the MCP server
    let mut server = GuiMcpServer::new(client, config);
    server.run().await?;

    Ok(())
}
