use clap::Parser;

#[derive(Parser, Debug)]
pub struct EncryptArgs {
    /// Path to the file to encrypt
    #[arg(short, long, value_name = "INPUT")]
    pub input_file: String,

    /// Optional: the output path of the encrypted file.
    /// By default the file will be written in the "encrypted"
    /// folder with the same name
    #[arg(short, long, value_name = "OUT", default_value = "")]
    pub output_file: String,
}