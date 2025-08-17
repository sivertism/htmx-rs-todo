use htmx_rs_todo::{database::Database, AppState, create_app};
use anyhow::Context;
use tokio::net::TcpListener;
use tracing_subscriber;
use tracing::info;
use clap::Parser;

/// Crappy todo app to test out HTMX with Rust as the backend
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Data storage directory
    #[arg(long, default_value = ".")]
    data_dir: std::path::PathBuf,

    /// Listening port
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Listening address
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt::init();

    let dbpath = cli.data_dir.join("todos.db");
    let photos_dir = cli.data_dir.join("photos");

    info!("Opening database at {:?}", dbpath);
    info!("Photos directory: {:?}", photos_dir);

    // Create photos directory if it doesn't exist
    if !photos_dir.exists() {
        std::fs::create_dir_all(&photos_dir).context("Create photos directory")?;
    }

    let db = Database::new(dbpath).await.context("Create db")?;

    let state = AppState { db, photos_dir };

    let app = create_app(state);

    // Bind a TCP listener to the specified address
    let listen_address = format!("{}:{}", cli.address, cli.port);
    let listener = TcpListener::bind(listen_address).await?;
    info!("listening on {}", listener.local_addr().unwrap());

    // Start the Axum server with the defined routes
    axum::serve(listener, app).await?;

    Ok(())
}