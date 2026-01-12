use std::{collections::HashMap, sync::Arc};

use crate::{CpkFile, utf_table::UTFTable};

mod column;

use column::ColumnDescriptor;

#[derive(Debug, Default)]
pub struct TocParser {
    string_pool: HashMap<Vec<u8>, Arc<str>>
}

impl TocParser {
    pub(crate) fn find(&self, table: &UTFTable) -> Option<(u64, u64)> {
        const INVALID: usize = usize::MAX;

        let mut toc_col = INVALID;
        let mut content_col = INVALID;

        let mut columns = Vec::with_capacity(table.metadata.column_count as usize);

        let string_pool = &table.data[table.metadata.string_pool_offset as usize..];
        let mut col_ptr = table.metadata.first_column_pos();

        for col_index in 0..table.metadata.column_count as usize {
            let column = ColumnDescriptor::new(table.data[col_ptr as usize]);
            let mut col_size: u32 = 1;

            if column.has_name() {
                let name_offset = column.string_offset(&table.data[col_ptr as usize..]);
                let name = read_utf_string(string_pool, name_offset as usize);

                if name == "TocOffset" {
                    toc_col = col_index;
                } else if name == "ContentOffset" {
                    content_col = col_index;
                }

                col_size += std::mem::size_of::<u32>() as u32;
            }

            if column.has_default() {
                col_size += column.value_len() as u32;
            }

            columns.push(column);
            col_ptr += col_size;
        }

        let mut toc_offset = None;
        let mut content_offset = None;

        let mut row_ptr = table.metadata.first_row_offset() as usize;

        for (i, column) in columns.iter().enumerate() {
            if i == toc_col {
                toc_offset = Some(column.read_number(&table.data[row_ptr..]));
            } else if i == content_col {
                content_offset = Some(column.read_number(&table.data[row_ptr..]));
            }

            if toc_offset.is_some() && content_offset.is_some() {
                break;
            }

            if column.is_row_storage() {
                row_ptr += column.value_len() as usize;
            }
        }

        let toc = toc_offset.expect("Unable to find TOC Offset in the CPK") as u64;
        let mut content = content_offset.expect("Unable to find Content Offset in the CPK") as u64;

        if toc < content {
            content = toc;
        }

        Some((toc, content))
    }

    pub(crate) fn read(&mut self, table: &UTFTable, content_offset: u64) -> Vec<CpkFile> {
        const INVALID: usize = usize::MAX;

        let mut dir_name_col_idx     = INVALID;
        let mut file_name_col_idx    = INVALID;
        let mut file_size_col_idx    = INVALID;
        let mut extract_size_col_idx = INVALID;
        let mut file_offset_col_idx  = INVALID;
        let mut user_string_col_idx  = INVALID;

        let string_pool = &table.data[table.metadata.string_pool_offset as usize..];
        let mut columns = Vec::with_capacity(table.metadata.column_count as usize);
        let mut col_ptr = table.metadata.first_column_pos();
        let mut defaults = Vec::with_capacity(table.metadata.column_count as usize);

        for col_index in 0..table.metadata.column_count as usize {
            let column = ColumnDescriptor::new(table.data[col_ptr as usize]);
            let mut col_size: u32 = 1;

            if column.has_name() {
                let name_offset = column.string_offset(&table.data[col_ptr as usize..]);
                let name = read_utf_string(string_pool, name_offset as usize);

                match name.as_str() { 
                    "DirName" => dir_name_col_idx = col_index,
                    "FileName" => file_name_col_idx = col_index,
                    "FileSize" => file_size_col_idx = col_index,
                    "ExtractSize" => extract_size_col_idx = col_index,
                    "FileOffset" => file_offset_col_idx = col_index,
                    "UserString" => user_string_col_idx = col_index,
                    _ => {},
                }
                
                col_size += std::mem::size_of::<u32>() as u32;
            }
            let default_val_offset = (col_ptr + col_size) as usize;

            if column.has_default() {
                let len = column.value_len() as usize;
                // Extract the actual bytes of the default value now
                let default_bytes = table.data[default_val_offset..default_val_offset + len].to_vec();
                defaults.push(default_bytes);

                col_size += column.value_len() as u32;
            } else {
                defaults.push(Vec::new());
            }

            columns.push(column);
            col_ptr += col_size;
        }

        // We now read the data
        let mut result: Vec<CpkFile> = Vec::with_capacity(table.metadata.row_count as usize);
        let base_row_pointer = table.metadata.first_row_offset(); // The first row is the header

        for row in 0..table.metadata.row_count {
            let mut cpk_file = CpkFile::default();

            let mut current_row_read_offset = (base_row_pointer + (row * table.metadata.row_size_bytes as u32)) as usize;
            
            for (col_idx, column) in columns.iter().enumerate() {
                // 1. Determine the source of the raw bytes for this cell
                // We use an Option<&[u8]> to hold the slice of data we want to read
                let cell_data: Option<&[u8]> = if column.is_row_storage() {
                    let len = column.value_len() as usize;
                    let slice = &table.data[current_row_read_offset..current_row_read_offset + len];

                    if current_row_read_offset + len > table.data.len() {
                        panic!("Read overflow: Row {}, Col {} out of bounds", row, col_idx);
                    }
                    
                    // IMPORTANT: Only advance the offset if we read from the row!
                    current_row_read_offset += len; 
                    Some(slice)
                } else if column.has_default() {
                    // Read from the defaults array instead of table.data
                    Some(&defaults[col_idx])
                } else {
                    None
                };

                if let Some(data) = cell_data {
                    if col_idx == dir_name_col_idx {
                        let string_offset = u32::from_be_bytes(data.try_into().unwrap());
                        cpk_file.directory = Some(self.get_or_insert(&string_pool[string_offset as usize..]));
                    } else if col_idx == file_name_col_idx {
                        let string_offset = u32::from_be_bytes(data.try_into().unwrap());
                        cpk_file.file_name = read_utf_string(&string_pool, string_offset as usize);
                    } else if col_idx == file_size_col_idx {
                        cpk_file.file_size = column.read_number(&data) as u32;
                    } else if col_idx == extract_size_col_idx {
                        cpk_file.extract_size = column.read_number(&data) as u32;
                    } else if col_idx == file_offset_col_idx {
                        cpk_file.file_offset = column.read_number(&data) as u64 + content_offset;
                    } else if col_idx == user_string_col_idx {
                        let string_offset = u32::from_be_bytes(data.try_into().unwrap());
                        cpk_file.user_string = Some(self.get_or_insert(&string_pool[string_offset as usize..]));
                    }
                }
            }
            result.push(cpk_file);
        }
        result
    }

    fn get_or_insert(&mut self, bytes: &[u8]) -> Arc<str> {
        self.string_pool.entry(bytes.to_vec())
            .or_insert_with(|| {
                let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                let bytes = &bytes[..end];
                Arc::<str>::from(str::from_utf8(bytes).unwrap())
            })
            .clone()
    }
}

/// Reads a string from a CRI UTF table string pool.
///
/// `string_pool` = slice containing the string pool bytes  
/// `offset` = offset into the string pool (from `CriString`)  
pub fn read_utf_string(string_pool: &[u8], offset: usize) -> String {
    // The string starts at offset
    let sub = &string_pool[offset..];

    // Find the null terminator
    let end = sub.iter().position(|&b| b == 0).unwrap_or(sub.len());
    let bytes = &sub[..end];

    String::from_utf8_lossy(bytes).into_owned()
}