use anyhow::Result;
use clap::{Parser, Subcommand};
use relaykit::{backend::BackendClient, config::Config, telegram::TelegramClient};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Parser)]
#[command(name = "relaykit")]
#[command(about = "Language-neutral communication gateway for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        #[arg(short, long, default_value = "relaykit.toml")]
        config: PathBuf,
    },
    Telegram {
        #[command(subcommand)]
        command: TelegramCommand,
    },
}

#[derive(Debug, Subcommand)]
enum TelegramCommand {
    Send {
        #[arg(short, long, default_value = "relaykit.toml")]
        config: PathBuf,
        #[arg(long)]
        chat_id: String,
        #[arg(long)]
        text: String,
        #[arg(long)]
        thread_id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("relaykit=info".parse()?))
        .init();

    match Cli::parse().command {
        Command::Serve { config } => {
            let config = Config::load(&config)?;
            let telegram = TelegramClient::from_config(&config.telegram)?;
            let backend = BackendClient::new(config.backend)?;
            relaykit::serve::run(telegram, backend).await
        }
        Command::Telegram { command } => match command {
            TelegramCommand::Send {
                config,
                chat_id,
                text,
                thread_id,
            } => {
                let config = Config::load(&config)?;
                let telegram = TelegramClient::from_config(&config.telegram)?;
                telegram
                    .send_message(&chat_id, thread_id.as_deref(), &text, None)
                    .await?;
                Ok(())
            }
        },
    }
}
