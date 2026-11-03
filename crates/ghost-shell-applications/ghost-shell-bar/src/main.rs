mod app;
mod blur;
mod services;

#[snafu::report]
fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    app::run()
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Could not run Ghost bar"))]
    Runtime { source: iced_exwlshell::Error },
}
