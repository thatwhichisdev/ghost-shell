mod app;
mod greeter;

use clap::Parser;

#[derive(Parser)]
#[command(about = "Ghost graphical greeter for greetd")]
struct Arguments {
    #[arg(long, default_value = "/etc/ghost-shell/greeter.toml")]
    config: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    app::run(&Arguments::parse().config)
}
