use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct DumpArgs {
    /// Path to the game's folder containing CPK files
    #[arg(short, long, value_name = "INPUT")]
    pub input_folder: String,

    /// Optional: the output folder where the files will be dumped
    #[arg(short, long, value_name = "OUT", default_value = "extracted")]
    pub output_folder: PathBuf,

    /// Optional: the total amount of threads allocated to the program.
    /// A value of 0 will use all available threads
    #[arg(short, long, value_name = "THREADS", default_value = "0")]
    pub threads: usize,

    /// Optional: The amount of memory the program is allowed to use in GiB.
    /// A value of 0 will use the full available memory.
    #[arg(short, long, value_name = "MEMORY", default_value = "0")]
    pub memory: f64,

    /// Optional: A text file with regex rules for selecting files that need
    /// extracting
    #[arg(short, long, value_name = "RULES_FILE", default_value = "")]
    pub rules_file: String,
}