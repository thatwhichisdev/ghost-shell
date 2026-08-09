use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::{env, path::PathBuf};

use ghost_shell_ipc::{
    FinderAction,
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
    Finder {
        #[arg(value_enum)]
        action: FinderCommand,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LauncherCommand {
    Toggle,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FinderCommand {
    Toggle,
}

/// Entry point for the command line interface.
///
/// Parses commands and communicates with `ghost-shell-daemon` using IPC socket.
///
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let socket_path =
        env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap();
    let mut client =
        Client::connect(socket_path.join("ghost-shell-daemon")).await?;

    let request = match cli.command {
        Command::Msg { command } => match command {
            MsgCommand::Launcher { action } => match action {
                LauncherCommand::Toggle => Request::Launcher {
                    action: LauncherAction::Toggle,
                },
            },
            MsgCommand::Finder { action } => match action {
                FinderCommand::Toggle => Request::Finder {
                    action: FinderAction::Toggle,
                },
            },
        },
    };

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
