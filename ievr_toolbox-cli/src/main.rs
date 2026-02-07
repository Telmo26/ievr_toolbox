use clap::Parser;

mod memory_budget;
mod args;
mod dump;
mod decrypt;
mod encrypt;

use args::{
    Args,
    Command,
};

pub use crate::{
    args::DumpArgs,
    args::DecryptArgs,
    args::EncryptArgs,
};

use dump::dump;
use decrypt::decrypt;
use encrypt::encrypt;

const TMP_PATH: &str = "temp";

const MB: usize = 1024 * 1024;
const GB: usize = 1024 * MB;

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Dump(dump_args) => dump(dump_args),
        Command::Decrypt(decrypt_args) => decrypt(decrypt_args),
        Command::Encrypt(encrypt_args) => encrypt(encrypt_args),
    }
    
}