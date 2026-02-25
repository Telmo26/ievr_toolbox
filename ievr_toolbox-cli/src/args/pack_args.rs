use clap::Parser;

#[derive(Parser, Debug)]
pub struct PackArgs {
    /// Path to the mod folder
    #[arg(short, long, value_name = "INPUT")]
    pub input_folder: String,

    /// The path to the vanilla CPK
    #[arg(short, long, value_name = "CPK")]
    pub vanilla_cpk: String,
}