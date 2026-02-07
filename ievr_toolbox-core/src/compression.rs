use std::{fs::OpenOptions, io, path::PathBuf};

mod utils;
mod reverse_bit_reader;

use memmap2::MmapMut;
pub use utils::is_compressed;

use reverse_bit_reader::ReverseBitReader;

use crate::cpk_file::CpkFile;

/// Constants defined in the original algorithm
const UNCOMPRESSED_DATA_SIZE: usize = 0x100;
const MIN_COPY_LENGTH: usize = 3;

#[derive(Debug, Default)]
pub struct Decompressor {}

impl Decompressor {
    pub fn decompress(&mut self, extracted_file_path: &PathBuf, extracted_file: &CpkFile) -> std::io::Result<()> {
        let decompressed_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(extracted_file_path)?;

        decompressed_file.set_len(extracted_file.extract_size as u64)?;

        let mut mmap = unsafe {
            MmapMut::map_mut(&decompressed_file)?
        };

        if let Some(compressed_data) = extracted_file.data() {
            decompress_layla(compressed_data, &mut mmap)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Decompression failed"))?;
        }

        Ok(())
    }

}


fn decompress_layla(compressed_data: &[u8], output: &mut MmapMut) -> Option<()> {
    if compressed_data.len() < 0x10 {
        return None;
    }
    
    // uncompSizeOfCompData is at offset 8 (u32 LE)
    let uncomp_size_of_comp_data = u32::from_le_bytes(compressed_data[8..12].try_into().unwrap()) as usize;
    // uncompHeaderOffset is at offset 12 (u32 LE)
    let uncomp_header_offset = u32::from_le_bytes(compressed_data[12..16].try_into().unwrap()) as usize;

    let total_output_size = uncomp_size_of_comp_data + UNCOMPRESSED_DATA_SIZE;

    // assert_eq!(
    //     output.len(), 
    //     total_output_size,
    //     "Extract size mismatch: header says {}, metadata says {}",
    //     total_output_size,
    //     output.len()
    // );

    let header_src_start = uncomp_header_offset + 0x10;

    if header_src_start + UNCOMPRESSED_DATA_SIZE > compressed_data.len() {
        return None; // Out of bounds safety
    }

    // Copy the header to the start of the output
    output[0..UNCOMPRESSED_DATA_SIZE]
        .copy_from_slice(&compressed_data[header_src_start..header_src_start + UNCOMPRESSED_DATA_SIZE]);

    let mut reader = ReverseBitReader::new(compressed_data, header_src_start);

    // We start reading from the back of the file
    let mut write_index = total_output_size.saturating_sub(1);
    let min_addr = UNCOMPRESSED_DATA_SIZE;

    while write_index >= min_addr {
        let is_compressed = reader.read_bit() == 1;

        if is_compressed {
            let offset = (reader.read_bits(13) as usize) + MIN_COPY_LENGTH;
            let mut length = MIN_COPY_LENGTH;

            let mut this_level = reader.read_bits(2) as usize;
            length += this_level;

            // Level 1
            if this_level == 3 {
                this_level = reader.read_bits(3) as usize;
                length += this_level;

                // Level 2
                if this_level == 7 {
                    this_level = reader.read_bits(5) as usize;
                    length += this_level;

                    // Level 3
                    if this_level == 31 {
                        loop {
                            this_level = reader.read_bits(8) as usize;
                            length += this_level;
                            if this_level != 255 {
                                break;
                            }
                        }
                    }
                }
            }

            for _ in 0..length {
                let src_idx = write_index + offset;

                if src_idx >= output.len() {
                   // In standard LZ77, this shouldn't happen with valid data.
                   // However, return None or break to avoid panic.
                   return None; 
                }
                
                output[write_index] = output[src_idx];
                
                if write_index == 0 { break; } // Safety for usize underflow
                write_index -= 1;
                
                // If we've written enough, break early (safety vs C# loop condition)
                if write_index < min_addr { break; }
            }
        } else {
            // Verbatim Byte
            let byte = reader.read_bits(8) as u8;
            output[write_index] = byte;
            
            if write_index == 0 { break; }
            write_index -= 1;
        }
    }
    Some(())
}