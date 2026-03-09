use std::{fs, path::PathBuf};

use crate::args::EncryptArgs;

const ENCRYPTED_PATH: &str = "encrypted";

pub fn encrypt(args: EncryptArgs) -> std::io::Result<()> {
    let file_path_str = args.input_file.trim_matches('"').trim_end_matches("\\");

    let file_path = PathBuf::from(file_path_str);

    let file_name = file_path.file_name().unwrap();

    let output_path = match args.output_file {
        Some(output_path) => PathBuf::from(output_path.trim_matches('"').trim_end_matches("\\")),
        None => PathBuf::from(ENCRYPTED_PATH).join(file_name),
    };

    if let Some(folder) = output_path.parent() {
        fs::create_dir_all(folder)?;
    }

    let result = ievr_toolbox_core::encrypt(&file_path, &output_path);

    match &result {
        Ok(()) => println!("File successfully encrypted to {}", output_path.display()),
        Err(e) => println!("File encryption failed due to {e}"),
    };

    result
}