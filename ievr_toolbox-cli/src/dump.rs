use crossbeam;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use std::{
    collections::BinaryHeap, fs::{self, DirBuilder, File}, io::{self, BufRead, BufReader}, path::{Path, PathBuf}, process::exit, thread, time::Instant
};

use ievr_cfg_bin_editor_core::{Database, Value, parse_database};

use crate::{GB, MB, TMP_PATH, args::DumpArgs, memory_budget::MemoryPool};

use ievr_toolbox_core::{
    CpkFile, Decompressor, DecryptedCpk, TocParser, decompress_files, decrypt_cpk,
    extract_cpk_files,
};

pub fn dump(args: DumpArgs) -> std::io::Result<()> {
    // Access the folder path
    let game_path = args.input_folder.trim_matches('"').trim_end_matches("\\"); // This removes all quotes and trailing backslashes

    let mut game_folder = PathBuf::from(game_path);

    if !game_folder.exists() {
        eprintln!("Error: The path {} does not exist.", game_folder.display());
        std::process::exit(1);
    }

    println!("Scanning game folder: {}", game_folder.display());

    if !game_folder.ends_with("data") {
        game_folder.push("data");
    }

    let mut dir_builder = DirBuilder::new();
    dir_builder.recursive(true);

    let temp_folder = PathBuf::from(TMP_PATH);
    if temp_folder.exists() {
        fs::remove_dir_all(&temp_folder).unwrap();
    }
    
    dir_builder.create(TMP_PATH)?;
    
    let extract_folder = &args.output_folder;
    if !extract_folder.exists() {
        dir_builder.create(extract_folder)?;
    }

    let mut files_to_process = Vec::new();
    visit_dirs(&game_folder, &mut |path| {
        if let Some(ext) = path.extension() {
            if ext.to_string_lossy().to_lowercase() == "cpk" {
                files_to_process.push(path);
            }
        }
    })?;

    let mut selected_files = Vec::new();
    if args.rules_file != "" {
        let rules_file_path = PathBuf::from(args.rules_file.trim_matches('"').trim_end_matches("\\")); // This removes all quotes and trailing backslashes
        let rules_file = File::open(rules_file_path).unwrap();

        let mut cpk_list_path = game_folder.to_path_buf();
        cpk_list_path.push("cpk_list.cfg.bin");

        let cpk_list = decrypt_cpk(&cpk_list_path, &temp_folder, 1 * GB);

        let cpk_list_database = parse_database(&cpk_list).unwrap();        

        (files_to_process, selected_files) = select_requested_cpks(cpk_list_database, files_to_process, rules_file);
    }
    // We sort the work by biggest files first

    files_to_process.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap());
    files_to_process.reverse();

    let total_files = files_to_process.len() as u64;
    let total_file_size: u64 = files_to_process
        .iter()
        .map(|path| fs::metadata(path).unwrap().len())
        .sum();

    println!(
        "Found {} CPK files ({:.2} GiB) to extract. Starting extraction...\n",
        total_files,
        total_file_size as f64 / GB as f64
    );

    // We compute the number of threads allocated to the program

    let max_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);

    let threads_in_use = if args.threads < 1 || args.threads > max_threads {
        max_threads
    } else {
        args.threads
    };

    let (decrypt_threads, extract_threads, decompress_threads) = compute_threads(threads_in_use);

    // We compute the memory limits based on the memory allocated to the program

    let system = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );

    let available_memory = system.available_memory() as usize;

    let memory = if args.memory == 0.0 || args.memory * GB as f64 > available_memory as f64 {
        available_memory
    } else {
        (args.memory * GB as f64) as usize
    };    

    let size_threshold = memory / decrypt_threads / 2; // We want to avoid the situation where the CPK + the files it contains go over the limit
    let memory_pool = MemoryPool::new(memory);

    println!("Memory allocated: {:.2} GiB - In-RAM decryption threshold: {} MiB\n", 
        memory as f64 / GB as f64,
        size_threshold / MB,
    );

    // We display the current settings

    println!(
        "Decryption threads: {}",
        decrypt_threads,
    );
    println!("Extraction threads: {:?}", extract_threads);
    println!(
        "Decompression threads: {}\n",
        decompress_threads,
    );

    // We create the channels that will be used to communicate

    let (dec_tx, dec_rx) = crossbeam::channel::unbounded::<DecryptedCpk>();
    let (ext_tx, ext_rx) = crossbeam::channel::unbounded::<CpkFile>();

    // We store the handles to the threads to be able to wait for them to finish

    let mut decrypt_handles = Vec::with_capacity(decrypt_threads);
    let mut extract_handles = Vec::with_capacity(extract_threads);
    let mut decompress_handles = Vec::with_capacity(decompress_threads);

    // We setup the progress bars

    let start_time = Instant::now();

    let mp = MultiProgress::new();

    let decryption_pb = mp.add(ProgressBar::new(total_file_size));
    decryption_pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} Decrypting CPKs [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
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
        let memory_pool = memory_pool.clone();

        decrypt_handles.push(thread::spawn(move || {
            for original_file in cpk_files.iter().skip(i).step_by(decrypt_threads) {
                let file_size = fs::metadata(original_file).unwrap().len() as usize;
                if file_size < size_threshold {
                    // This will map the file to RAM instead of a file
                    memory_pool.acquire_decryption(file_size as usize);
                }

                let decrypted_cpk = decrypt_cpk(&original_file, &temp_folder, size_threshold);

                decrypt_pb.inc(file_size as u64);

                tx.send(decrypted_cpk).unwrap();
            }
        }));
    }

    // Extractor thread
    {
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
                                if selected_files.is_empty() || selected_files.contains(&extracted_file.file_name) {
                                    heap.push(extracted_file);
                                }
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
                                    if selected_files.is_empty() || selected_files.contains(&extracted_file.file_name) {
                                        heap.push(extracted_file);
                                    }
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
        let memory_pool = memory_pool.clone();

        decompress_handles.push(thread::spawn(move || {
            let mut decompressor = Decompressor::default();
            while let Ok(extracted_file) = ext_rx.recv() {
                if extracted_file.extract_size as usize > memory_pool.limit() {
                    extract_pb.finish_and_clear();
                    eprintln!("Insufficient memory allocation for decompression, aborting...");
                    exit(1);
                }
                memory_pool.acquire_decompression(extracted_file.extract_size as usize);

                decompress_files(&mut decompressor, &extracted_file, &extract_folder);

                memory_pool.release(extracted_file.extract_size as usize);

                let file_size = extracted_file.cpk_size().unwrap();
                if let Some(true) = extracted_file.last_cpk_file() && file_size < size_threshold {
                    // If it has been mapped to RAM
                    memory_pool.release(file_size);
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
        let memory_pool = memory_pool.clone();

        decompress_handles.push(thread::spawn(move || {
            let mut decompressor = Decompressor::default();
            while let Ok(extracted_file) = ext_rx.recv() {
                if extracted_file.extract_size as usize > memory_pool.limit() {
                    extract_pb.finish_and_clear();
                    eprintln!("Insufficient memory allocation for decompression, aborting...");
                    exit(1);
                }
                memory_pool.acquire_decompression(extracted_file.extract_size as usize);

                decompress_files(&mut decompressor, &extracted_file, &extract_folder);

                memory_pool.release(extracted_file.extract_size as usize);

                let file_size = extracted_file.cpk_size().unwrap();
                if extracted_file.last_cpk_file().unwrap() && file_size < size_threshold {
                    // If it has been mapped to RAM
                    memory_pool.release(file_size);
                }

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

    fs::remove_dir_all(temp_folder).unwrap();

    let duration = start_time.elapsed();

    println!("\n--- Extraction Summary ---");
    println!("Total time: {:.2?}", duration);

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
    // We only want 1 extraction thread because extraction is so fast,
    // it doesn't copy anything. It simply extracts metadata from
    // the decrypted CPK and reorganizes it
    let extract_threads = 1;

    let decrypt_threads = threads_in_use / 2;
    let decompress_threads = threads_in_use - decrypt_threads - extract_threads;

    (decrypt_threads, extract_threads, decompress_threads.max(1))
}

fn select_requested_cpks(cpk_list: Database, mut cpk_files: Vec<PathBuf>, rules_file: File) -> (Vec<PathBuf>, Vec<String>) {
    let mut selected_cpk = Vec::new();
    let mut selected_files = Vec::new();

    let buf_reader = BufReader::new(rules_file);

    let cpk_table = cpk_list.table("CPK_ITEM").unwrap();

    let lines = buf_reader.lines();
    for regex in lines.map_while(Result::ok) {
        let re = match Regex::new(&regex) {
            Ok(re) => re,
            Err(_) => {
                eprintln!("Invalid regex {regex}, ignoring it...");
                continue
            }
        };

        for row in cpk_table.rows() {
            if row.name.contains("BEG") || row.name.contains("END") { continue; }
            
            let file_name = match &row.values[1][0] {
                Value::String(s) => s.clone(),
                _ => continue,
            };

            let cpk_name = match &row.values[3][0] {
                Value::String(s) => s.clone(),
                _ => continue,
            };

            if re.is_match(&file_name) {
                selected_files.push(file_name.clone());
                selected_cpk.push(cpk_name.clone());
            }

        }
    };

    cpk_files = cpk_files.into_iter().filter(|cpk_file| {
        let filename = cpk_file.file_name().unwrap().to_str().unwrap();
        selected_cpk.iter().any(|s| s == filename)
    }).collect();

    (cpk_files, selected_files)
}