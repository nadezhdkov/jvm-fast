use clap::Parser;
use jvmfast::cli::{run, Cli};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    run(Cli::parse()).await
}
