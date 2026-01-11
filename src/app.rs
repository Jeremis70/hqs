use crate::cli::{Cli, Cmd};

pub fn run(cli: Cli) -> i32 {
    match cli.cmd {
        Cmd::Capture(_args) => {
            println!("Hello, World");
            0
        }
    }
}
