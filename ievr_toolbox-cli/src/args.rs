use clap::{Parser, Subcommand};

mod dump_args;
mod decrypt_args;
mod encrypt_args;
mod pack_args;

pub use self::{
    dump_args::DumpArgs,
    decrypt_args::DecryptArgs,
    encrypt_args::EncryptArgs,
    pack_args::PackArgs,
};

#[derive(Parser, Debug)]
#[command(author, version, about = "IE VR Toolbox", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Extract files from CPK archives
    Dump(DumpArgs),
    
    /// Decrypt CRIware encrypted file
    Decrypt(DecryptArgs),

    /// Encrypt files into CRIware
    Encrypt(EncryptArgs),

    /// Pack mod using vanilla CPK
    Pack(PackArgs)
}