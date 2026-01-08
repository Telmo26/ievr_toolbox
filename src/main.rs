use std::{fs::File, io::Seek, path::{Path, PathBuf}};

mod criware_crypt;
mod utf_table;
mod column;
mod toc;
mod cpk_file;

use criware_crypt::CriwareCrypt;
use utf_table::UTFTable;
use toc::{toc_finder, toc_reader};

fn main() -> std::io::Result<()> {
    let input_path = Path::new("9f44bbcbf647e12ee4c3b84e62dd8209.cpk");
    let output_path = with_suffix(input_path, "decrypted");

    let mut crypt_file = CriwareCrypt::new(input_path)?;
    
    let mut output_file = if Path::exists(&output_path) {
        File::open(output_path)?
    } else {
        let mut output_file = File::create(output_path)?;
        crypt_file.decrypt(&mut output_file)?;
        output_file
    };

    // Parse the master table
    let table = UTFTable::new(&mut output_file)?;

    let (toc_offset, content_offset) = toc_finder(&table)
        .ok_or_else(|| std::io::ErrorKind::InvalidData)?;

    println!("TOC Offset: {:X}", toc_offset);

    // Move the file to the beginning of the TOC table to parse it
    output_file.seek(std::io::SeekFrom::Start(toc_offset))?;
    let toc_table = UTFTable::new(&mut output_file)?;

    let files = toc_reader(&toc_table, content_offset);

    for file in files {
        println!("{:#?}", file)
    }

    Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new(""));

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap();
    let extension = path.extension().and_then(|s| s.to_str());

    let new_name = match extension {
        Some(ext) => format!("{file_stem}-{suffix}.{ext}"),
        None => format!("{file_stem}-{suffix}"),
    };

    parent.join(new_name)
}

