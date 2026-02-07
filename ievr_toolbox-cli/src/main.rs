use clap::Parser;

mod memory_budget;
mod args;
mod dump;

use args::{
    Args,
    Command,
};

pub use crate::{
    args::DumpArgs
};

use dump::dump;

const TMP_PATH: &str = "temp";

const MB: usize = 1024 * 1024;
const GB: usize = 1024 * MB;

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Dump(dump_args) => dump(dump_args),
    }
    
}