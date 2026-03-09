use clap::Parser;

mod args;
mod functions;
mod common;

use args::{
    Args,
    Command,
};

use crate::common::constants;

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if std::fs::exists(constants::TMP_PATH)? { 
        std::fs::remove_dir_all(constants::TMP_PATH)?;
    }
    std::fs::create_dir_all(constants::TMP_PATH)?;

    match args.command {
        Command::Dump(dump_args) => functions::dump(dump_args),
        Command::Decrypt(decrypt_args) => functions::decrypt(decrypt_args),
        Command::Encrypt(encrypt_args) => functions::encrypt(encrypt_args),
        Command::Pack(pack_args) => functions::pack(pack_args),
        Command::Merge(merge_args) => functions::merge(merge_args),
    }?;

    std::fs::remove_dir_all(constants::TMP_PATH)
}