use std::{fs::{self, File, OpenOptions}, io::{Seek, Write}, ops::Deref, path::{Path, PathBuf}, sync::Arc};

use memmap2::Mmap;

mod criware_crypt;
mod utf_table;
mod cpk_file;
mod compression;
mod toc_parser;

use criware_crypt::CriwareCrypt;
use utf_table::UTFTable;
use compression::is_compressed;

pub use crate::{
    toc_parser::TocParser,
    compression::Decompressor, 
    cpk_file::CpkFile
};

pub type DecryptedCpk = Arc<CpkData>;

#[derive(Debug)]
pub enum CpkData {
    Big(Mmap),
    Small(Vec<u8>)
}

impl Deref for CpkData {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            CpkData::Big(mmap) => &mmap,
            CpkData::Small(vec) => &vec,
        }
    }
}

pub fn dump_cpk(input_path: PathBuf, tmp_folder: &PathBuf, extract_folder: &PathBuf) {
    let decrypted_cpk = decrypt_cpk(&input_path, &tmp_folder, 256 * 1024 * 1024);

    let mut toc_parser = TocParser::default();
    let extracted_files = extract_cpk_files(decrypted_cpk, &mut toc_parser);

    let mut decompressor = Decompressor::default();

    for extracted_file in extracted_files {
        decompress_files(&mut decompressor, &extracted_file, extract_folder);
    }
}

pub fn decrypt_cpk(input_path: &PathBuf, tmp_folder: &PathBuf, size_threshold: usize) -> DecryptedCpk {
    let mut tmp_file_path = tmp_folder.clone();
    tmp_file_path.push(input_path.file_name().unwrap());

    let mut crypt_file = CriwareCrypt::new(&input_path)
        .expect("Unable to load the CPK file in the decryption module");
    
    let decrypted_cpk = if fs::metadata(input_path).unwrap().len() as usize >= size_threshold {
        let f = if Path::exists(&tmp_file_path) {
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
        Arc::new(CpkData::Big(unsafe { Mmap::map(&f).unwrap() }))
    } else {
        Arc::new(CpkData::Small(crypt_file.decrypt_ram().unwrap()))
    };

    // let decrypted_cpk = Arc::new(crypt_file.decrypt_ram().unwrap());

    // let decrypted_cpk = Arc::new(unsafe { Mmap::map(&decrypted_cpk).unwrap() });

    decrypted_cpk
}

pub fn extract_cpk_files(decrypted_cpk: DecryptedCpk, toc_parser: &mut TocParser) -> Vec<CpkFile> {
    // Parse the master master_table
    let master_table = UTFTable::new(&decrypted_cpk, 0)
        .expect("Unable to parse the master UTF master_table");

    let (toc_offset, content_offset) = toc_parser.find(&master_table)
        .expect("Unable to find the TOC");

    // Move the file to the beginning of the TOC master_table to parse it
    let toc_table = UTFTable::new(&decrypted_cpk, toc_offset as usize)
        .expect("Unable to parse the TOC");

    let mut extracted_files = toc_parser.read(&toc_table, content_offset);

    for file in &mut extracted_files {
        file.set_decrypted_cpk(&decrypted_cpk);
        
        if file.file_size > file.extract_size {
            eprintln!("File {}: error on file size computing", file.file_name)
        }
    }

    extracted_files
}

pub fn decompress_files(decompressor: &mut Decompressor, extracted_file: &CpkFile, extract_folder: &PathBuf) {
    let mut extracted_file_path = extract_folder.clone();
    if let Some(dir) = &extracted_file.directory {
        extracted_file_path.push(dir.as_ref());
    } 
    fs::create_dir_all(&extracted_file_path).unwrap();
    extracted_file_path.push(&extracted_file.file_name);
    
    if is_compressed(&extracted_file) {
        decompressor.decompress(&extracted_file_path, &extracted_file)
            .expect(&format!("Failed to decompress {}", extracted_file_path.to_string_lossy()));
    } else {
        let mut file_handle = File::create(extracted_file_path).unwrap();
        file_handle.write_all(&mut extracted_file.data().unwrap()).unwrap();
    }
}

pub fn decrypt(input_path: &Path, output_path: &Path) -> std::io::Result<()> {
    let mut crypt = CriwareCrypt::new(input_path)?;

    let mut output_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_path)
        .expect("Unable to open pre-existing temporary file");

    crypt.decrypt(&mut output_file)
}

pub fn encrypt(input_path: &Path, output_path: &Path) -> std::io::Result<()> {
    let mut crypt = CriwareCrypt::new(input_path)?;

    let mut output_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_path)
        .expect("Unable to open pre-existing temporary file");

    crypt.encrypt(&mut output_file)
}