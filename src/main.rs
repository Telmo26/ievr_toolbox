use indicatif::{ProgressBar, ProgressStyle};
use rayon::{ThreadPoolBuilder, prelude::*};
use clap::Parser;

use std::{fs::{self, DirBuilder}, io, path::{Path, PathBuf}, time::Instant};

use ie_vr_decrypt::extract_cpk;

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
    dir_builder.create("tmp")?;

    let extract_folder = &args.output;
    if !extract_folder.exists() {
        dir_builder.create(extract_folder)?;
    }

    let max_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap() / 2; // We only want to run a thread per CPU physical core, not logical one

    let num_threads = if args.threads < 1 || args.threads > max_cores {
        max_cores
    } else {
        args.threads
    };

    println!("Using {num_threads} thread(s) (System max: {max_cores})");

    let pool = ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();

    let mut files_to_process = Vec::new();
    visit_dirs(&args.input, &mut |path| {
        if let Some(ext) = path.extension() {
            if ext.to_string_lossy().to_lowercase() == "cpk" {
                files_to_process.push(path);
            }
        }
    })?;

    let total_files = files_to_process.len() as u64;
    println!("Found {} CPK files. Starting extraction...", total_files);

    let start_time = Instant::now();

    // 2. Initialize the Progress Bar
    let pb = ProgressBar::new(total_files);
    pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
    )
    .unwrap()
    .progress_chars("#>-"));

    pool.install(|| {
        files_to_process.into_par_iter().for_each(|file| {
            extract_cpk(file, &extract_folder);

            pb.inc(1);
        });
    });

    // files_to_process.into_iter().for_each(|file| {
    //     extract_cpk(file, &extract_folder);

    //     pb.inc(1);
    // });

    pb.finish_with_message("Done!");
    let duration = start_time.elapsed();

    println!("\n--- Extraction Summary ---");
    println!("Total time: {:.2?}", duration);
    println!("Files processed: {}", total_files);

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about = "CPK File Extractor", long_about = None)]
struct Args {
    /// Path to the game's folder containing CPK files
    #[arg(short, long, value_name = "INPUT")]
    input: PathBuf,

    // The output folder where the files will be dumped
    #[arg(short, long, value_name = "OUT", default_value = "extracted")]
    output: PathBuf,

    #[arg(short, long, value_name = "THREADS", default_value = "0")]
    threads: usize
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