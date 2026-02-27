mod errors;
mod server;
mod tools;

use rmcp::ServiceExt;
use server::ToadService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = match ToadService::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Failed to initialize Toad workspace: {}", e);
            eprintln!("Please ensure Toad is initialized with 'toad home' or set TOAD_HOME.");
            std::process::exit(1);
        }
    };

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let server = service.serve(transport).await?;
    server.waiting().await?;
    Ok(())
}
