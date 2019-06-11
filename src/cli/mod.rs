use std::{env, path::PathBuf};

use lazy_static::lazy_static;
use structopt::StructOpt;
use terminal_size as term;

use witnet_config as config;

mod node;
mod wallet;

pub fn from_args() -> Cli {
    Cli::from_args()
}

pub fn exec(command: Cli) -> Result<(), failure::Error> {
    match command {
        Cli {
            config,
            debug,
            trace,
            cmd,
            ..
        } => {
            let config = get_config(config.or_else(config::dirs::find_config))?;
            let mut log_opts = LogOptions {
                level: config.log.level,
                source: LogOptionsSource::Config,
            };

            if let Ok(rust_log) = env::var("RUST_LOG") {
                if rust_log.contains("witnet") {
                    log_opts = LogOptions {
                        level: env_logger::Logger::from_default_env().filter(),
                        source: LogOptionsSource::Env,
                    };
                }
            }

            if trace {
                log_opts = LogOptions {
                    level: log::LevelFilter::Trace,
                    source: LogOptionsSource::Flag,
                };
            } else if debug {
                log_opts = LogOptions {
                    level: log::LevelFilter::Debug,
                    source: LogOptionsSource::Flag,
                };
            }

            init_logger(log_opts);
            exec_cmd(cmd, config)
        }
    }
}

fn exec_cmd(command: Command, config: config::config::Config) -> Result<(), failure::Error> {
    match command {
        Command::Node(cmd) => node::exec_cmd(cmd, config),
        Command::Wallet(cmd) => wallet::exec_cmd(cmd, config),
    }
}

fn init_logger(opts: LogOptions) {
    println!(
        "Setting log level to: {}, source: {:?}",
        opts.level, opts.source
    );
    env_logger::Builder::from_env(env_logger::Env::default())
        .default_format_timestamp(true)
        .default_format_module_path(true)
        .filter_level(log::LevelFilter::Info)
        .filter_module("witnet", opts.level)
        .init();
}

fn get_config(path: Option<PathBuf>) -> Result<config::config::Config, failure::Error> {
    match path {
        Some(p) => {
            println!("Loading config from: {}", p.display());
            let config = config::loaders::toml::from_file(p)
                .map(|p| config::config::Config::from_partial(&p))?;
            Ok(config)
        }
        None => {
            println!("HEADS UP! No configuration specified/found. Using default one!");
            Ok(config::config::Config::default())
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(raw(max_term_width = "*TERM_WIDTH"))]
pub struct Cli {
    #[structopt(short = "c", long = "config", raw(help = "CONFIG_HELP"))]
    config: Option<PathBuf>,
    /// Turn on DEBUG logging.
    #[structopt(long = "debug")]
    debug: bool,
    /// Turn on TRACE logging.
    #[structopt(long = "trace")]
    trace: bool,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "node", about = "Witnet full node.")]
    Node(node::Command),
    #[structopt(name = "wallet", about = "Witnet wallet.")]
    Wallet(wallet::Command),
}

struct LogOptions {
    level: log::LevelFilter,
    source: LogOptionsSource,
}

#[derive(Debug)]
enum LogOptionsSource {
    Config,
    Env,
    Flag,
}

lazy_static! {
    static ref TERM_WIDTH: usize = {
        let size = term::terminal_size();
        if let Some((term::Width(w), _)) = size {
            w as usize
        } else {
            120
        }
    };
}

static CONFIG_HELP: &str =
    r#"Load configuration from this file. If not specified will try to find a configuration
in these paths:
- current path
- standard configuration path:
  - $XDG_CONFIG_HOME/witnet/witnet.toml in Gnu/Linux
  - $HOME/Library/Preferences/witnet/witnet.toml in MacOS
  - C:\Users\<YOUR USER>\AppData\Roaming\witnet\witnet.toml
- /etc/witnet/witnet.toml if in a *nix platform
If no configuration is found. The default configuration is used, see `config` subcommand if
you want to know more about the default config."#;