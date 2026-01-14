use clap::Parser;
use crossbeam;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use std::{
    collections::BinaryHeap, fs::{self, DirBuilder}, io, path::{Path, PathBuf}, thread, time::Instant
};

mod memory_budget;

use memory_budget::MemoryBudget;

use ievr_toolbox::{
    self, CpkFile, Decompressor, DecryptedCpk, TocParser, decompress_files, decrypt_cpk,
    extract_cpk_files,
};

const TMP_PATH: &str = "temp";

const MB: usize = 1024 * 1024;
const GB: usize =  1024 * MB;

#[derive(Parser, Debug)]
#[command(author, version, about = "CPK File Extractor", long_about = None)]
struct Args {
    /// Path to the game's folder containing CPK files
    #[arg(short, long, value_name = "INPUT")]
    input: PathBuf,

    // The output folder where the files will be dumped
    #[arg(short, long, value_name = "OUT", default_value = "extracted")]
    output: PathBuf,

    // The total amount of cores allocated to the program. A default value of 0 will
    // use all available cores
    #[arg(short, long, value_name = "CORES", default_value = "0")]
    cores: usize,
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

    files_to_process.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap());
    files_to_process.reverse();

    let total_files = files_to_process.len() as u64;
    let total_file_size: u64 = files_to_process.iter().map(|path| {
        fs::metadata(path).unwrap().len()
    })
    .sum();

    println!("Found {} CPK files ({} GiB). Starting extraction...", total_files, total_file_size / GB as u64);

    // We compute the number of threads allocated to the program

    let max_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);

    let threads_in_use = if args.cores < 1 || args.cores > max_threads {
        max_threads
    } else {
        args.cores
    };

    let (decrypt_threads, extract_threads, decompress_threads) = compute_threads(threads_in_use);

    // We compute the memory limits based on the memory allocated to the program

    let system = System::new_with_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()));

    let memory = system.available_memory() as usize;

    println!("Total memory: {memory}");

    let size_threshold = memory / decrypt_threads;
    let cpk_budget = MemoryBudget::new(memory * 2/3);
    let decompress_budget = MemoryBudget::new(memory * 1/3);

    // We display the current settings

    println!("Decryption threads: {} - Memory allocated: {} GiB", decrypt_threads, (memory * 2/3) / GB);
    println!("Extraction threads: {:?}", extract_threads);
    println!("Decompression threads: {} - Memory allocated: {} GiB", decompress_threads, (memory / 3) / GB);

    // We create the channels that will be used to communicate

    let (dec_tx, dec_rx) = crossbeam::channel::bounded::<DecryptedCpk>(2 * decrypt_threads);
    let (ext_tx, ext_rx) = crossbeam::channel::bounded::<CpkFile>(2 * decompress_threads);

    // We store the handles to the threads to be able to wait for them to finish

    let mut decrypt_handles = Vec::with_capacity(decrypt_threads);
    let mut extract_handles = Vec::with_capacity(extract_threads);
    let mut decompress_handles = Vec::with_capacity(decompress_threads);

    // We setup the progress bars

    let start_time = Instant::now();

    let mp = MultiProgress::new();

    let decryption_pb = mp.add(ProgressBar::new(total_file_size));
    decryption_pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} Decrypting files [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
    )
    .unwrap()
    .progress_chars("#>-"));
    decryption_pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let extract_pb = mp.add(ProgressBar::new(0));
    extract_pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} Extracting files [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
    )
    .unwrap()
    .progress_chars("#>-"));
    extract_pb.enable_steady_tick(std::time::Duration::from_millis(100));

    for i in 0..decrypt_threads {
        let tx = dec_tx.clone();
        let cpk_files = files_to_process.clone();
        let temp_folder = temp_folder.clone();
        let decrypt_pb = decryption_pb.clone();
        let cpk_budget = cpk_budget.clone();

        decrypt_handles.push(thread::spawn(move || {
            for original_file in cpk_files.iter().skip(i).step_by(decrypt_threads) {
                let file_size = fs::metadata(original_file).unwrap().len() as usize;
                if file_size < size_threshold { // This will map the file to RAM instead of a file
                    cpk_budget.acquire(file_size as usize);
                }

                let decrypted_cpk = decrypt_cpk(&original_file, &temp_folder, size_threshold);

                decrypt_pb.inc(file_size as u64);

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
            let mut heap = BinaryHeap::<CpkFile>::new();
            let mut extraction_done = false;

            loop {
                if extraction_done {
                    while let Some(extracted_file) = heap.pop() {
                        extract_pb.inc_length(extracted_file.extract_size as u64);
                        // If receiver hangs up, stop sending
                        if ext_tx.send(extracted_file).is_err() {
                            break;
                        }
                    }
                    break;
                }

                // Phase 2: If Heap is Empty, we MUST block.
                // If we don't block here, we hit the 'default' branch below instantly and spin the CPU.
                if heap.is_empty() {
                    match dec_rx.recv() {
                        Ok(decrypted_file) => {
                            for extracted_file in extract_cpk_files(decrypted_file, &mut toc_parser) {
                                heap.push(extracted_file);
                            };
                        }
                        Err(_) => extraction_done = true,
                    }
                    continue;
                }

                // Phase 3: Heap has items, and Input is still open.
                // We prioritize checking for new data (to keep the pipe full), 
                // but default to sending data if no new messages are ready.
                crossbeam::select! {
                    recv(dec_rx) -> msg => {
                        match msg {
                            Ok(decrypted_file) => {
                                for extracted_file in extract_cpk_files(decrypted_file, &mut toc_parser) {
                                    heap.push(extracted_file);
                                }
                            },
                            Err(_) => extraction_done = true,
                        }
                    }
                    default => {
                        // We know heap is not empty because of the check in Phase 2
                        if let Some(extracted_file) = heap.pop() {
                            extract_pb.inc_length(extracted_file.extract_size as u64);
                            if ext_tx.send(extracted_file).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }));
    }

    for _ in 0..decompress_threads {
        let ext_rx = ext_rx.clone();
        let extract_folder = extract_folder.clone();
        let extract_pb = extract_pb.clone();
        let decompress_budget: MemoryBudget = decompress_budget.clone();
        let cpk_budget = cpk_budget.clone();

        decompress_handles.push(thread::spawn(move || {
            let mut decompressor = Decompressor::default();
            while let Ok(extracted_file) = ext_rx.recv() {
                decompress_budget.acquire(extracted_file.extract_size as usize);

                decompress_files(&mut decompressor, &extracted_file, &extract_folder);

                decompress_budget.release(extracted_file.extract_size as usize);

                let file_size = extracted_file.cpk_size().unwrap();
                if extracted_file.last_cpk_file().unwrap() && file_size < size_threshold { // If it has been mapped to RAM
                    cpk_budget.release(file_size);
                }
                
                extract_pb.inc(extracted_file.extract_size as u64);
            }
        }));
    }

    for dec_handle in decrypt_handles {
        dec_handle.join().unwrap();
    }

    decryption_pb.finish();

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

    extract_pb.finish();

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
    let extract_threads = 1.max(threads_in_use / 8); // We enable two extraction threads for 8-cores CPUs

    let decrypt_threads = threads_in_use / 3;
    let decompress_threads = threads_in_use - decrypt_threads - extract_threads;

    (decrypt_threads, extract_threads, decompress_threads.max(1))
}
