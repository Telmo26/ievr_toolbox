use std::{fs, path::PathBuf};

use crate::DecryptArgs;

const DECRYPTED_PATH: &str = "decrypted";

pub fn decrypt(args: DecryptArgs) -> std::io::Result<()> {
    let file_path_str = args.input_file.trim_matches('"').trim_end_matches("\\");

    let file_path = PathBuf::from(file_path_str);

    let file_name = file_path.file_name().unwrap();

    let output_path = if args.output_file.is_empty() {
        PathBuf::from(DECRYPTED_PATH).join(file_name)
    } else {
        PathBuf::from(
            args.output_file.trim_matches('"').trim_end_matches("\\")
        )
    };

    if let Some(folder) = output_path.parent() {
        fs::create_dir_all(folder)?;
    }

    ievr_toolbox_core::decrypt(&file_path, &output_path)
}