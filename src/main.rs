use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use clap::Parser;
use crossbeam;

use std::{fs::{self, DirBuilder}, io, path::{Path, PathBuf}, thread, time::Instant};

use ievr_toolbox::{self, CpkFile, Decompressor, DecryptedCpk, TocParser, decompress_files, decrypt_cpk, extract_cpk_files};

const TMP_PATH: &str = "temp";

#[derive(Parser, Debug)]
#[command(author, version, about = "CPK File Extractor", long_about = None)]
struct Args {
    /// Path to the game's folder containing CPK files
    #[arg(short, long, value_name = "INPUT")]
    input: PathBuf,

    // The output folder where the files will be dumped
    #[arg(short, long, value_name = "OUT", default_value = "extracted")]
    output: PathBuf,

    // The total amount of threads allocated to the program. A default value of 0 will
    // use all available threads
    #[arg(short, long, value_name = "THREADS", default_value = "0")]
    threads: usize
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Access the folder path
    let game_folder = &args.input;
    if !game_folder.exists() {
        eprintln!("Error: The path {:?} does not exist.", game_folder);
        std::process::exit(1);
    }

    println!("Scanning game folder: {:?}", game_folder);

    let mut dir_builder = DirBuilder::new();
    dir_builder.recursive(true);

    dir_builder.create(TMP_PATH)?;
    let temp_folder = PathBuf::from(TMP_PATH);

    let extract_folder = &args.output;
    if !extract_folder.exists() {
        dir_builder.create(extract_folder)?;
    }

    let mut files_to_process = Vec::new();
    visit_dirs(&args.input, &mut |path| {
        if let Some(ext) = path.extension() {
            if ext.to_string_lossy().to_lowercase() == "cpk" {
                files_to_process.push(path);
            }
        }
    })?;

    // We sort the work by biggest files first
    files_to_process.sort_by_key(|p| {
        std::fs::metadata(p).map(|m| m.len()).unwrap()
    });
    files_to_process.reverse();

    let total_files = files_to_process.len() as u64;
    println!("Found {} CPK files. Starting extraction...", total_files);

    let max_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);

    let threads_in_use = if args.threads < 1 || args.threads > max_threads {
        max_threads
    } else {
        args.threads
    };

    let (decrypt_threads, extract_threads, decompress_threads) = compute_threads(threads_in_use);

    println!("Decryption threads: {:?}", decrypt_threads);
    println!("Extraction threads: {:?}", extract_threads);
    println!("Decompression threads: {:?}", decompress_threads);

    let (dec_tx, dec_rx) = crossbeam::channel::unbounded::<DecryptedCpk>();
    let (ext_tx, ext_rx) = crossbeam::channel::unbounded::<CpkFile>();

    let mut decrypt_handles = Vec::with_capacity(decrypt_threads);
    let mut extract_handles = Vec::with_capacity(extract_threads);
    let mut decompress_handles = Vec::with_capacity(decompress_threads);

    let start_time = Instant::now();

    let mp = MultiProgress::new();

    let decryption_pb = mp.add(ProgressBar::new(total_files));
    decryption_pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} Decrypting files [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
    )
    .unwrap()
    .progress_chars("#>-"));
    decryption_pb.enable_steady_tick(std::time::Duration::from_millis(300));

    let extract_pb = mp.add(ProgressBar::new(0));
    extract_pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} Extracting files [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
    )
    .unwrap()
    .progress_chars("#>-"));
    extract_pb.enable_steady_tick(std::time::Duration::from_millis(300));

    for i in 0..decrypt_threads {
        let tx = dec_tx.clone();
        let cpk_files = files_to_process.clone();
        let temp_folder = temp_folder.clone();
        let decrypt_pb = decryption_pb.clone();

        decrypt_handles.push(thread::spawn(move || {
            for original_file in cpk_files.iter().skip(i).step_by(decrypt_threads) {
                // let mut mmap = unsafe { MmapMut::map_mut(&file).unwrap() };

                let decrypted_cpk = decrypt_cpk(&original_file, &temp_folder);

                decrypt_pb.inc(1);

                tx.send(decrypted_cpk).unwrap();
            }
        }));
    }

    for _ in 0..extract_threads {
        let dec_rx = dec_rx.clone();
        let ext_tx = ext_tx.clone();
        let extract_pb = extract_pb.clone();

        extract_handles.push(thread::spawn(move || {
            let mut toc_parser = TocParser::default();
            while let Ok(decrypted_cpk) = dec_rx.recv() {
                let extracted_files = extract_cpk_files(decrypted_cpk, &mut toc_parser);
                
                for extracted_file in extracted_files {
                    extract_pb.inc_length(extracted_file.extract_size as u64);
                    ext_tx.send(extracted_file).unwrap();
                }
            }
        }));
    }

    for _ in 0..decompress_threads {
        let ext_rx = ext_rx.clone();
        let extract_folder = extract_folder.clone();
        let extract_pb = extract_pb.clone();

        decompress_handles.push(thread::spawn(move || {
            let mut decompressor = Decompressor::default();
            while let Ok(extracted_file) = ext_rx.recv() {
                decompress_files(&mut decompressor, &extracted_file, &extract_folder);
                extract_pb.inc(extracted_file.extract_size as u64);
            }
        }));
    }

    for dec_handle in decrypt_handles {
        dec_handle.join().unwrap();
    }

    decryption_pb.finish_with_message("Done!");

    drop(dec_tx);

    for _ in 0..decrypt_threads {
        let ext_rx = ext_rx.clone();
        let extract_folder = extract_folder.clone();
        let extract_pb = extract_pb.clone();

        decompress_handles.push(thread::spawn(move || {
            let mut decompressor = Decompressor::default();
            while let Ok(extracted_file) = ext_rx.recv() {
                decompress_files(&mut decompressor, &extracted_file, &extract_folder);
                extract_pb.inc(extracted_file.extract_size as u64);
            }
        }));
    }

    for ext_handle in extract_handles {
        ext_handle.join().unwrap();
    }

    drop(ext_tx);

    for decomp_handle in decompress_handles {
        decomp_handle.join().unwrap();
    }

    extract_pb.finish_with_message("Done!");

    let duration = start_time.elapsed();

    println!("\n--- Extraction Summary ---");
    println!("Total time: {:.2?}", duration);
    println!("Files processed: {}", total_files);

    Ok(())
}

fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(PathBuf)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Recursively call the function for subdirectories
                visit_dirs(&path, cb)?;
            } else {
                // It's a file, execute the callback
                cb(path);
            }
        }
    }
    Ok(())
}

fn compute_threads(threads_in_use: usize) -> (usize, usize, usize) {
    // We only want 1 extraction thread because it is so fast, 
    // it doesn't copy anything. It simply extracts metadata from 
    // the decrypted CPK and reorganizes it
    let extract_threads = 1; 

    let decrypt_threads = threads_in_use.div_ceil(2);
    let decompress_threads = threads_in_use - decrypt_threads - 1;

    (decrypt_threads, extract_threads, decompress_threads.max(1))
}