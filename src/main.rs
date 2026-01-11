mod app;
mod cli;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    let code = app::run(cli);
    if code != 0 {
        std::process::exit(code);
    }
}
