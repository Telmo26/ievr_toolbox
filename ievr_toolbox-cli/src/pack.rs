use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use ievr_cfg_bin_editor_core::{Database, T2b, Value, parse_database};
use ievr_toolbox_core::{decrypt_cpk, encrypt};
use walkdir::WalkDir;

use crate::args::PackArgs;

pub fn pack(pack_args: PackArgs) -> std::io::Result<()> {
    let (mut cpk_list, index_map) = parse_cpk_list(&pack_args.vanilla_cpk)?;

    let mut dir = PathBuf::from(&pack_args.input_folder);

    let files_to_process = WalkDir::new(&dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file());

    {
        let table = cpk_list.table_mut("CPK_ITEM").unwrap();
        let rows = table.rows_mut();

        for file in files_to_process {
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

    dir.push("cpk_list.cfg.bin");

    T2b::write(cpk_list.into());

    encrypt(Path::new("output"), &dir)?;

    fs::remove_file("output")?;

    Ok(())
}

fn parse_cpk_list(vanilla_cpk: &str) -> std::io::Result<(Database, HashMap<PathBuf, usize>)> {
    let input_path = PathBuf::from(vanilla_cpk.trim_matches('"').trim_end_matches("\\"));
    let tmp_folder = PathBuf::new();

    let decrypted_cpk_list = decrypt_cpk(&input_path, &tmp_folder, 1_000_000_000);
    let cpk_list = parse_database(&decrypted_cpk_list)?;
    let cpk_item_table = cpk_list.table("CPK_ITEM").unwrap();

    let mut cpk_item_iter = cpk_item_table.rows().into_iter();

    let header = cpk_item_iter.next().unwrap();
    let nb_entries = if let Value::Int(v) = header.values[0][0] { v } else { unreachable!() } as usize;

    let mut string_map = HashMap::with_capacity_and_hasher(nb_entries, Default::default());

    for (index, row) in cpk_item_iter.enumerate() {
        let directory = match &row.values[0][0] {
            Value::String(s) => s,
            _ => continue,
        };

        let file_name = match &row.values[1][0] {
            Value::String(s) => s,
            _ => continue,
        };

        let mut full = PathBuf::with_capacity(directory.len() + file_name.len() + 1);
        full.push(directory);
        full.push(file_name);
        
        string_map.insert(full, index+1);
    }

    Ok((cpk_list, string_map))
}