mod app;
mod greeter;

use clap::Parser;

#[derive(Parser)]
#[command(about = "Ghost graphical greeter for greetd")]
struct Arguments {
    #[arg(long, default_value = "/etc/ghost-shell/greeter.toml")]
    config: std::path::PathBuf,
}

#[snafu::report]
fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    app::run(&Arguments::parse().config)
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Could not run Ghost greeter"))]
    Runtime { source: iced_exwlshell::Error },
    #[snafu(display("Could not load greeter configuration"))]
    Configuration {
        source: ghost_shell_config::ConfigError,
    },
    #[snafu(display(
        "session.command must contain a nonempty command without NUL bytes"
    ))]
    InvalidSessionCommand,
    #[snafu(display(
        "session.environment must contain NAME=value entries without NUL bytes"
    ))]
    InvalidSessionEnvironment,
}
