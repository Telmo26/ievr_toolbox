use std::{fs, path::PathBuf};

use crate::EncryptArgs;

const ENCRYPTED_PATH: &str = "encrypted";

pub fn encrypt(args: EncryptArgs) -> std::io::Result<()> {
    let file_path_str = args.input_file.trim_matches('"').trim_end_matches("\\");

    let file_path = PathBuf::from(file_path_str);

    let file_name = file_path.file_name().unwrap();

    let output_path = if args.output_file.is_empty() {
        PathBuf::from(ENCRYPTED_PATH).join(file_name)
    } else {
        PathBuf::from(
            args.output_file.trim_matches('"').trim_end_matches("\\")
        )
    };

    if let Some(folder) = output_path.parent() {
        fs::create_dir_all(folder)?;
    }

    ievr_toolbox_core::encrypt(&file_path, &output_path)
}