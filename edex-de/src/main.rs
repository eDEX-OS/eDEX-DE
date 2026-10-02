//! eDEX-DE: the shell process. `edex-de run` draws the panels on Hyprland's layer shell;
//! `edex-de ipc …` talks to a running shell.

mod app;
mod events;
mod forms;
mod input;
mod ipc_handler;
mod overlays;
mod status;
mod toolkits;

use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "edex-de",
    version,
    about = "eDEX-DE desktop shell for Hyprland"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Configuration file (default: $XDG_CONFIG_HOME/edex-de/config.toml).
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Do not connect to the Hyprland sockets (for running under another compositor).
    #[arg(long, global = true)]
    no_hypr: bool,
    /// Run for N seconds, print a JSON report and exit (CI).
    #[arg(long, global = true, value_name = "SECS")]
    smoke_test: Option<u64>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run the shell (default).
    Run,
    /// Send a command to the running shell: `toggle launcher`, `audio volume +5`, `state`, …
    Ipc {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.cmd {
        Some(Cmd::Ipc { args }) => ipc_cli(&args),
        _ => {
            init_logging();
            match app::run(app::RunOptions {
                config_path: cli.config.unwrap_or_else(settings::config_path),
                no_hypr: cli.no_hypr,
                smoke: cli.smoke_test.map(Duration::from_secs),
            }) {
                Ok(code) => code,
                Err(e) => {
                    tracing::error!("fatal: {e:#}");
                    eprintln!("edex-de: {e:#}");
                    1
                }
            }
        }
    };
    std::process::exit(code);
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

fn ipc_cli(args: &[String]) -> i32 {
    let request = match ipc::proto::parse_args(args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("edex-de ipc: {e}");
            return 2;
        }
    };
    match ipc::send(&ipc::socket_path(), &request) {
        Ok(resp) => {
            println!("{}", serde_json::to_string(&resp).unwrap_or_default());
            if resp.ok {
                0
            } else {
                1
            }
        }
        Err(e) => {
            // Outside an eDEX session (e.g. a folder opened from another desktop) `files` still
            // opens ranger, in kitty.
            if let ipc::proto::Request::Files { path } = &request {
                let mut cmd = std::process::Command::new("kitty");
                cmd.args(["--class", "ranger", "-e", "ranger"]);
                cmd.args(path.iter());
                if cmd.spawn().is_ok() {
                    return 0;
                }
            }
            eprintln!("edex-de ipc: {e:#}");
            3
        }
    }
}

pub fn app_terminal_config(app: &app::App) -> terminal::TerminalConfig {
    app::terminal_config(&app.config)
}

#[allow(dead_code)]
fn _assert_result(_: Result<()>) {}
