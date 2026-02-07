use clap::{Parser, Subcommand};

mod dump_args;
pub use dump_args::DumpArgs;

#[derive(Parser, Debug)]
#[command(author, version, about = "IE VR Toolbox", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Extract files from CPK archives
    Dump(DumpArgs)
}