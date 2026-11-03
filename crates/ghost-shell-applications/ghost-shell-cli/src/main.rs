use clap::{Parser, Subcommand, ValueEnum};
use ghost_shell_ipc::{BarAction, Endpoint, LockAction, Request};
use snafu::ResultExt as _;

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
    Bar {
        #[arg(value_enum)]
        action: BarCommand,
    },
    Shutdown {
        #[arg(value_enum)]
        application: Application,
    },

    Session {
        #[arg(value_enum)]
        action: SessionCommand,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BarCommand {
    Open,
    Close,
    Toggle,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SessionCommand {
    Lock,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Application {
    Bar,
    Lockscreen,
}

impl From<Application> for Endpoint {
    fn from(application: Application) -> Self {
        match application {
            Application::Bar => Self::Bar,
            Application::Lockscreen => Self::Lockscreen,
        }
    }
}

#[snafu::report]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let endpoint = match &cli.command {
        Command::Msg { command } => match command {
            MsgCommand::Bar { .. } => Endpoint::Bar,
            MsgCommand::Session { .. } => Endpoint::Lockscreen,
            MsgCommand::Shutdown { application } => (*application).into(),
        },
    };
    let request = match cli.command {
        Command::Msg { command } => match command {
            MsgCommand::Shutdown { .. } => Request::Shutdown,
            MsgCommand::Bar { action } => Request::Bar {
                action: match action {
                    BarCommand::Open => BarAction::Open,
                    BarCommand::Close => BarAction::Close,
                    BarCommand::Toggle => BarAction::Toggle,
                },
            },
            MsgCommand::Session { action } => match action {
                SessionCommand::Lock => Request::Lock {
                    action: LockAction::Lock,
                },
            },
        },
    };

    let reply = ghost_shell_ipc::call(endpoint, request)
        .await
        .context(ContactSnafu {
            endpoint: endpoint.name(),
        })?;

    match reply {
        Ok(response) => {
            println!("{response:?}");
        }

        Err(message) => {
            return Err(Error::Rejected { message });
        }
    }

    Ok(())
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Could not contact {endpoint}; is its service running?"))]
    Contact {
        endpoint: &'static str,
        source: ghost_shell_ipc::Error,
    },
    #[snafu(display("Daemon rejected the request: {message}"))]
    Rejected { message: String },
}
