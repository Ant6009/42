use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "42", about = "Standalone local-network Perplexity clone")]
struct Cli {
    /// Path to the TOML config file.
    #[arg(short, long, default_value = "/etc/42/42.toml")]
    config: std::path::PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP server (default when no subcommand is given).
    Serve,
    /// Administrative operations.
    Admin {
        #[command(subcommand)]
        action: AdminAction,
    },
}

#[derive(Subcommand)]
enum AdminAction {
    /// Create a user (prompts for a password).
    CreateUser { username: String },
    /// Print the argon2id hash of a password (for the admin seed in TOML).
    HashPassword,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fortytwo=info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let config = fortytwo::config::Config::load(&cli.config)?;

    match cli.command {
        None | Some(Command::Serve) => {
            info!(bind = %config.server.bind, "starting 42");
            fortytwo::server::run(config).await
        }
        Some(Command::Admin { action }) => match action {
            AdminAction::CreateUser { username } => {
                fortytwo::auth::cli_create_user(&config, &username)
            }
            AdminAction::HashPassword => fortytwo::auth::cli_hash_password(),
        },
    }
}
