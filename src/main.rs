mod error;
mod grammar;
mod output;

use clap::{Args, CommandFactory, Parser, Subcommand};
use relaykit::{backend::BackendClient, config::Config, telegram::TelegramClient};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tracing_subscriber::{EnvFilter, fmt};

use error::{Result, other, target};
use output::{OutputFormat, Response};

#[derive(Parser)]
#[command(
    name = "relaykit",
    version,
    about = "Language-neutral communication gateway for AI agents"
)]
struct Cli {
    /// Output format
    #[arg(long, global = true, help_heading = "Global Options", value_enum, default_value_t = OutputFormat::Json)]
    output: OutputFormat,
    /// Print debug logs to stderr
    #[arg(long, global = true, help_heading = "Global Options")]
    debug: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the gateway between the messaging platform and the backend
    #[command(subcommand)]
    Gateway(GatewayCommand),
    /// Call the Telegram Bot API directly
    #[command(subcommand)]
    Telegram(TelegramCommand),
}

#[derive(Subcommand)]
enum GatewayCommand {
    /// Poll Telegram updates, post each normalized event to the backend, and send its replies until interrupted
    #[command(name = "start-gateway")]
    Start(ConfigFileArgs),
}

#[derive(Subcommand)]
enum TelegramCommand {
    /// Send one Telegram message
    #[command(name = "send-message")]
    Send(SendMessageArgs),
}

#[derive(Args)]
struct ConfigFileArgs {
    /// Path of the RelayKit TOML config
    #[arg(long, default_value = "relaykit.toml")]
    config_file: PathBuf,
}

#[derive(Args)]
struct SendMessageArgs {
    /// Path of the RelayKit TOML config
    #[arg(long, default_value = "relaykit.toml")]
    config_file: PathBuf,
    /// Telegram chat ID
    #[arg(long)]
    chat_id: String,
    /// Message text
    #[arg(long)]
    text: String,
    /// Telegram forum topic ID (message_thread_id)
    #[arg(long)]
    thread_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendMessageReport {
    chat_id: String,
    thread_id: Option<String>,
    message_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct StartGatewayReport {
    stopped_by: String,
}

fn main() {
    let code = match run() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            error.exit_code()
        }
    };
    std::process::exit(code);
}

fn to_value<T: Serialize>(value: &T) -> Result<Value> {
    serde_json::to_value(value).map_err(|error| other(error.to_string()))
}

fn target_error(error: anyhow::Error) -> error::Error {
    target(format!("{error:#}"))
}

fn load(config_file: &Path) -> Result<Config> {
    Config::load(config_file).map_err(target_error)
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| other(error.to_string()))
}

fn run() -> Result<()> {
    let mut root = Cli::command();
    root.build();
    let argv = grammar::prepare(&root, std::env::args_os().collect())?;
    let debug = argv.iter().any(|token| token == "--debug");

    let cli = match Cli::try_parse_from(&argv) {
        Ok(cli) => cli,
        Err(error) => {
            let code = if error.use_stderr() { 252 } else { 0 };
            let _ = error.print();
            std::process::exit(code);
        }
    };

    let directive = if debug {
        "relaykit=debug"
    } else {
        "relaykit=info"
    };
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                directive
                    .parse()
                    .map_err(|error| other(format!("{error}")))?,
            ),
        )
        .init();
    if debug {
        tracing::debug!(argv = ?&argv[1..], "parsed arguments");
    }

    let (name, value) = match cli.command {
        Command::Gateway(GatewayCommand::Start(args)) => {
            let config = load(&args.config_file)?;
            let telegram = TelegramClient::from_config(&config.telegram).map_err(target_error)?;
            let backend = BackendClient::new(config.backend).map_err(target_error)?;
            runtime()?
                .block_on(relaykit::serve::run(telegram, backend))
                .map_err(target_error)?;
            let report = StartGatewayReport {
                stopped_by: "SIGINT".to_owned(),
            };
            ("start-gateway", to_value(&report)?)
        }
        Command::Telegram(TelegramCommand::Send(args)) => {
            let config = load(&args.config_file)?;
            let telegram = TelegramClient::from_config(&config.telegram).map_err(target_error)?;
            let message = runtime()?
                .block_on(telegram.send_message(
                    &args.chat_id,
                    args.thread_id.as_deref(),
                    &args.text,
                    None,
                ))
                .map_err(target_error)?;
            let report = SendMessageReport {
                chat_id: args.chat_id,
                thread_id: args.thread_id,
                message_id: message.get("message_id").map(|id| match id {
                    Value::String(id) => id.clone(),
                    id => id.to_string(),
                }),
            };
            ("send-message", to_value(&report)?)
        }
    };
    print!(
        "{}",
        output::render(
            &grammar::pascal_case(name),
            Response { value, text: None },
            cli.output
        )?
    );
    Ok(())
}
