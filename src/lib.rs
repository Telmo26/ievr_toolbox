use std::{fs::{self, File, OpenOptions}, io::{BufReader, Read, Seek}, path::{Path, PathBuf}};

mod criware_crypt;
mod utf_table;
mod column;
mod toc;
mod cpk_file;
mod compression;

use criware_crypt::CriwareCrypt;
use utf_table::UTFTable;
use toc::{toc_finder, toc_reader};
use compression::is_compressed;

use crate::{compression::{Decompressor, replace_prefix}, cpk_file::CpkFile};

const TMP_FOLDER: &str = "tmp";

pub fn extract_cpk(input_path: PathBuf, extract_folder: &PathBuf) {
    let tmp_folder = PathBuf::from(TMP_FOLDER);

    let mut tmp_file_path = tmp_folder.clone();
    tmp_file_path.push(input_path.file_name().unwrap());

    let mut crypt_file = CriwareCrypt::new(&input_path)
        .expect("Unable to load the CPK file in the decryption module");
    
    let mut tmp_file = if Path::exists(&tmp_file_path) {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&tmp_file_path)
            .expect("Unable to create output file")
    } else {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_file_path)
            .expect("Unable to open pre-existing temporary file");
        
        crypt_file.decrypt(&mut f)
            .expect("Unable to decrypt file");

        f.rewind().expect("Unable to rewind output file after decryption"); 
        f
    };

    return;

    // Parse the master master_table
    let master_table = UTFTable::new(&mut tmp_file)
        .expect("Unable to parse the master UTF master_table");

    let (toc_offset, content_offset) = toc_finder(&master_table)
        .expect("Unable to find the TOC");

    // Move the file to the beginning of the TOC master_table to parse it
    tmp_file.seek(std::io::SeekFrom::Start(toc_offset)).unwrap();
    let toc_table = UTFTable::new(&mut tmp_file)
        .expect("Unable to parse the TOC");

    let files = toc_reader(&toc_table, content_offset);

    let extracted_files = extract_cpk_files(tmp_file, files, &tmp_folder);

    // Remove temporary file
    std::fs::remove_file(tmp_file_path)
        .expect("Unable to remove temporary file");

    let mut decompressor = Decompressor::default();

    for extracted_file_path in extracted_files {
        let mut file_desc = File::open(&extracted_file_path).unwrap(); // We know this file exists since we wrote it previously
        
        if is_compressed(&mut file_desc).unwrap() {
            decompressor.decompress(&extracted_file_path, &mut file_desc, extract_folder)
                .expect(&format!("Failed to decompress {}", extracted_file_path.to_string_lossy()));
        } else {
            // We replace the temporary path with the extraction path
            let final_path = replace_prefix(&extracted_file_path, extract_folder);

            // We create the directory structure if it doesn't exist
            let final_parent = final_path.parent().unwrap();
            std::fs::create_dir_all(final_parent).unwrap();

            // We move the files from the temporary folder to the extraction folder
            std::fs::rename(extracted_file_path, final_path)
                .expect("Failure when moving non-compressed file");
        }
    }
}

fn extract_cpk_files(tmp_file: File, files: Vec<CpkFile>, tmp_folder: &PathBuf) -> Vec<PathBuf> {
    let mut buffered_reader = BufReader::with_capacity(128 * 1024, tmp_file);

    let mut extracted_files = Vec::with_capacity(files.len());

    for file in files {
        let mut file_path = tmp_folder.clone();
        if let Some(dir_path) = file.directory {
            let dir_path = Path::new(&dir_path);
            file_path.push(dir_path);

            fs::create_dir_all(&file_path)
                .expect("Unable to create the directory structure");
        } 
        file_path.push(&file.file_name);


        if let Ok(ref mut new_file) = File::create_new(&file_path) {
            buffered_reader.seek(std::io::SeekFrom::Start(file.file_offset)).unwrap();

            let mut file_handle = std::io::Read::by_ref(&mut buffered_reader).take(file.file_size as u64);

            std::io::copy(&mut file_handle, new_file)
                .expect("Extraction of individual files failed unexpectedly");
        }
        
        if file.file_size > file.extract_size {
            eprintln!("File {}: error on file size computing", file.file_name)
        }

        extracted_files.push(file_path);
    }

    extracted_files
}