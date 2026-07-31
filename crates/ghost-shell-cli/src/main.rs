use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::{env, path::PathBuf};

use ghost_shell_ipc::{
    client::Client,
    protocol::{LauncherAction, Request},
};

#[derive(Debug, Parser)]
#[command(name = "ghost-shell")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Msg {
        #[command(subcommand)]
        command: MsgCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum MsgCommand {
    Launcher {
        #[arg(value_enum)]
        action: LauncherCommand,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LauncherCommand {
    Toggle,
}

/// Entry point for the command line interface.
///
/// Parses commands and communicates with `ghost-shell-daemon` using IPC socket.
///
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let request = match cli.command {
        Command::Msg { command } => match command {
            MsgCommand::Launcher { action } => match action {
                LauncherCommand::Toggle => Request::Launcher {
                    action: LauncherAction::Toggle,
                },
            },
        },
    };

    let ipc_socket_path =
        env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap();
    let mut client =
        Client::connect(ipc_socket_path.join("ghost-shell-daemon")).await?;

    client.write(request).await?;
    let reply = client.read().await?;

    match reply {
        Ok(response) => {
            println!("{response:?}");
        }

        Err(message) => {
            bail!("{message}");
        }
    }

    Ok(())
}
