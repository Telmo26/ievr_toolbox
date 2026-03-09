use std::{fs::File, path::{Path, PathBuf}};

use walkdir::{DirEntry, WalkDir};
use rayon::prelude::*;

use crate::{args::{MergeArgs, PackArgs}, functions};

pub fn merge(args: MergeArgs) -> std::io::Result<()> {
    let mods_folder = PathBuf::from(args.mods_folder.trim_matches('"').trim_end_matches("\\"));
    let output_folder = PathBuf::from(args.output_folder.trim_matches('"').trim_end_matches("\\"));

    std::fs::create_dir_all(&output_folder)?;

    let mod_files: Vec<DirEntry> = WalkDir::new(&mods_folder)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect();

    mod_files.into_par_iter()
        .map(|file| {
            copy_file(&file, &mods_folder, &output_folder)
        })
        .collect::<std::io::Result<Vec<()>>>()?;

    let pack_args = PackArgs {
        input_folder: args.output_folder,
        vanilla_cpk: args.vanilla_cpk,
        output_folder: None
    };

    return functions::pack(pack_args);
}

fn copy_file(file: &DirEntry, mods_folder: &Path, output_folder: &Path) -> std::io::Result<()> {
    let file_relative_path = get_file_relative_path(&file, mods_folder);
    let os_str = file_relative_path.as_os_str();
    
    if os_str == "" || os_str == "mod_data.json" || os_str.to_string_lossy().contains("cpk_list.cfg.bin") {
        return Ok(());
    }
    
    let mut input_file = File::open(file.path())?;

    let output_path = output_folder.join(file_relative_path);

    let parent_folder = output_path.parent().unwrap();
    if !std::fs::exists(parent_folder)? { std::fs::create_dir_all(parent_folder)?; }

    let mut output_file = File::create(output_path)?;

    std::io::copy(&mut input_file, &mut output_file)?;

    Ok(())
}

fn get_file_relative_path(file: &DirEntry, mods_folder: &Path) -> PathBuf {
    let relative_mod_path = file.path().strip_prefix(mods_folder).unwrap(); // This is the relative path to the mods folder;

    // The goal is to iterate over the components, and remove the first one that is the mod folder itself
    relative_mod_path.into_iter() 
        .skip(1)
        .collect()
}