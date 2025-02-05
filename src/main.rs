mod config;
mod confluence;
mod error;
mod markdown;
use clap::{Parser, ValueEnum};
use config::Config;
use confluence::{ConfluenceClient, UpdatePageTrait};
use std::sync::OnceLock;
use tracing::{error, span, Level};

#[derive(Parser, Debug)]
#[command(
    name = "confluence-updater",
    about,
    version,
    after_help = "Author: Patrick Kerwood <patrick@kerwood.dk>",
    arg_required_else_help = true
)]
struct CommandArgs {
    #[arg(short, long, env = "CU_USER", help = "Confluence user to login with")]
    user: String,

    #[arg(
        short,
        long,
        env = "CU_SECRET",
        help = "The token/secret to use. https://id.atlassian.com/manage-profile/security/api-tokens"
    )]
    secret: String,

    #[arg(
        long,
        env = "CU_FQDN",
        help = "The fully qualified domain name of your Atlassian Cloud."
    )]
    fqdn: String,

    #[arg(
        short,
        long,
        default_value = "./confluence-updater.yaml",
        env = "CU_CONFIG_PATH",
        help = "The path to the config file."
    )]
    config_path: String,

    #[arg(
        name = "label",
        short,
        long = "label",
        help = "Add a label to all updating pages. Can be used multiple times."
    )]
    labels: Vec<String>,

    #[arg(long, env, default_value = "info", help = "Log Level.", global = true)]
    log_level: LogLevel,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

static FQDN: OnceLock<String> = OnceLock::new();
static USER: OnceLock<String> = OnceLock::new();
static SECRET: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() {
    let args = CommandArgs::parse();

    FQDN.set(args.fqdn.to_owned()).unwrap();
    USER.set(args.user.to_owned()).unwrap();
    SECRET.set(args.secret.to_owned()).unwrap();

    let log_level: Level = args.log_level.into();
    tracing_subscriber::fmt()
        .compact()
        .without_time()
        .with_max_level(log_level)
        .init();

    let config: Config = match args.try_into() {
        Ok(config) => config,
        Err(error) => {
            error!(%error);
            std::process::exit(1);
        }
    };

    let client = match ConfluenceClient::new(&config) {
        Ok(client) => client,
        Err(error) => {
            error!(%error);
            std::process::exit(1);
        }
    };

    for page in config.pages.iter() {
        let span = span!(
            Level::INFO,
            "page",
            id = page.page_id,
            title = page.title(),
            path = page.file_path,
            sha = page.sha(),
        );

        let _enter = span.enter();

        let _ = client.update_confluence_page(page).await.map_err(|error| {
            error!(%error);
            std::process::exit(1);
        });
    }
}
