use clap::Parser;

#[derive(Parser, Debug)]
pub struct DecryptArgs {
    /// Path to the file to decrypt
    #[arg(short, long, value_name = "INPUT")]
    pub input_file: String,

    /// Optional: the output path of the decrypted file.
    /// By default the file will be written in the "decrypted"
    /// folder with the same name
    #[arg(short, long, value_name = "OUT", default_value = "")]
    pub output_file: String,
}