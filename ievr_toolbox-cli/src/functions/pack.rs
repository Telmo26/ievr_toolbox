use std::{collections::HashMap, fs::File, path::PathBuf};

use ievr_cfg_bin_editor_core::{Database, Value, parse_database};
use ievr_toolbox_core::{decrypt_cpk, encrypt};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use walkdir::WalkDir;

use crate::{args::PackArgs, common::constants::TMP_PATH};

pub fn pack(args: PackArgs) -> std::io::Result<()> {
    let (mut cpk_list, index_map) = parse_cpk_list(&args.vanilla_cpk)?;

    let mut dir = PathBuf::from(&args.input_folder);

    let files_to_process: Vec<walkdir::DirEntry> = WalkDir::new(&dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect();

    // This block is needed so that references are automatically dropped
    {
        let table = cpk_list.table_mut("CPK_ITEM").unwrap();
        let rows = table.rows_mut();

        for file in files_to_process.iter() {
            let relative_path = file.path().strip_prefix(&dir).unwrap();
            
            if let Some(index) = index_map.get(relative_path) {
                let file_size = file.metadata()?.len();

                let row = &mut rows[*index];

                match &mut row.values[2][0] {
                    Value::String(s) => s.clear(),
                    _ => unreachable!()
                };

                match &mut row.values[3][0] {
                    Value::String(s) => s.clear(),
                    _ => unreachable!()
                };

                match &mut row.values[4][0] {
                    Value::Int(s) => *s = file_size as i32,
                    _ => unreachable!()
                };
            }
        }
    }

    if let Some(output_path) = args.output_folder {
        let new_dir = PathBuf::from(output_path);

        files_to_process.into_par_iter().for_each(|file| {
            let relative_path = file.path().strip_prefix(&dir).unwrap();

            let output_path = new_dir.join(relative_path);
            let parent = output_path.parent().unwrap();

            if !std::fs::exists(&parent).unwrap() { std::fs::create_dir_all(parent).unwrap() }

            let mut input_file = File::open(file.path()).unwrap();
            let mut output_file = File::create(output_path).unwrap();

            std::io::copy(&mut input_file, &mut output_file).unwrap();
        });

        dir = new_dir;
    } 

    dir.push("data/cpk_list.cfg.bin");
    let tmp_path = TMP_PATH.to_owned() + "/cpk_list.cfg.bin"; // We use the temp folder to write the unencrypted file
    cpk_list.write(&tmp_path)?; 

    encrypt(&tmp_path, &dir)?;

    Ok(())
}

fn parse_cpk_list(vanilla_cpk: &str) -> std::io::Result<(Database, HashMap<PathBuf, usize>)> {
    let input_path = PathBuf::from(vanilla_cpk.trim_matches('"').trim_end_matches("\\"));
    let tmp_folder = PathBuf::new();

    let decrypted_cpk_list = decrypt_cpk(&input_path, &tmp_folder, 1_000_000_000);
    let cpk_list = parse_database(&decrypted_cpk_list)?;
    let cpk_item_table = cpk_list.table("CPK_ITEM").unwrap();

    let cpk_item_rows = cpk_item_table.rows();

    let str_map = cpk_item_rows.into_iter()
        .enumerate()
        .skip(1)
        .map(|(index, row)| {
            let directory = if let Value::String(s) = &row.values[0][0] { s } else { unreachable!() };
            let file_name = if let Value::String(s) = &row.values[1][0] { s } else { unreachable!() };

            let mut full = PathBuf::with_capacity(directory.len() + file_name.len() + 1);
            full.push(directory);
            full.push(file_name);

            (full, index)
        })  
        .collect();

    Ok((cpk_list, str_map))
}