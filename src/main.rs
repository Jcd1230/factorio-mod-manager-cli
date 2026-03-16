mod app;
mod cli;
mod config;
mod domain;
mod error;
mod factorio;
mod portal_api;
mod ui;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    if let Err(error) = app::run(cli) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
