use clap::Parser;

#[derive(Parser, Debug)]
pub struct PackArgs {
    /// Path to the mod folder
    #[arg(short, long, value_name = "INPUT")]
    pub input_folder: String,

    /// The path to the vanilla CPK
    #[arg(short, long, value_name = "CPK")]
    pub vanilla_cpk: String,

    /// Optional: The path of the packed mod. If this is not specified,
    /// the resulting cpk_list.cfg.bin will be written inside the data
    /// folder of the input mod
    #[arg(short, long, value_name = "OUTPUT", default_value = None)]
    pub output_folder: Option<String>
}