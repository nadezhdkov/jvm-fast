use clap::Parser;

#[derive(Parser)]
#[command(
    name = "jvmfast",
    version,
    about = "jvm-fast: gerenciador de dependências e build para Java"
)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
