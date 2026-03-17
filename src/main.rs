use clap::Parser;

use factorio_mods_manager::cli::Cli;
use factorio_mods_manager::commands;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(error) = commands::run(cli).await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
