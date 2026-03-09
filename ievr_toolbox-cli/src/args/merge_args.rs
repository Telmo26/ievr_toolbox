use clap::Parser;

#[derive(Parser, Debug)]
pub struct MergeArgs {
    /// Path to the folder containing the mods to merge
    #[arg(short, long, value_name = "MODS")]
    pub mods_folder: String,

    /// Path to the output folder
    #[arg(short, long, value_name = "OUTPUT")]
    pub output_folder: String,

    /// The path to the vanilla CPK
    #[arg(short, long, value_name = "CPK")]
    pub vanilla_cpk: String,
}