use std::{fs::{self, File}, io::{self, BufReader, BufWriter, Read, Seek, Write}, path::{Path, PathBuf}};

mod utils;
mod reverse_bit_reader;

pub use utils::{is_compressed, replace_prefix};

use reverse_bit_reader::ReverseBitReader;

/// Constants defined in the original algorithm
const UNCOMPRESSED_DATA_SIZE: usize = 0x100;
const MIN_COPY_LENGTH: usize = 3;

#[derive(Debug, Default)]
pub struct Decompressor {
    input_data_buffer: Vec<u8>,
    output_data_buffer: Vec<u8>,
}

impl Decompressor {
    pub fn decompress(&mut self, extracted_file_path: &PathBuf, extracted_file: &File, extract_folder: &Path) -> std::io::Result<()> {
        let decompression_path = replace_prefix(extracted_file_path, extract_folder);
        let parent_dir = decompression_path.parent().unwrap();
        // Create the directory structure needed for decompression
        fs::create_dir_all(parent_dir)?;
        
        let decompressed_file = File::create(&decompression_path)
            .expect("Failed to create the decompression file");

        // We clear the input buffer
        self.input_data_buffer.clear();

        let mut buffered_reader = BufReader::with_capacity(128 * 1024, extracted_file);
        buffered_reader.seek(io::SeekFrom::Start(0))?; // We reset the cursor in the file
        buffered_reader.read_to_end(&mut self.input_data_buffer)?;

        // We clear the output buffer before decompressing
        self.output_data_buffer.clear();
        decompress_layla(&self.input_data_buffer, &mut self.output_data_buffer)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Decompression failed"))?;

        let mut buffered_writer = BufWriter::with_capacity(128 * 1024, decompressed_file);
        buffered_writer.write_all(&self.output_data_buffer)?;

        Ok(())
    }

}


fn decompress_layla(input: &[u8], output: &mut Vec<u8>) -> Option<()> {
    if input.len() < 0x10 {
        return None;
    }
    
    // uncompSizeOfCompData is at offset 8 (u32 LE)
    let uncomp_size_of_comp_data = u32::from_le_bytes(input[8..12].try_into().unwrap()) as usize;
    // uncompHeaderOffset is at offset 12 (u32 LE)
    let uncomp_header_offset = u32::from_le_bytes(input[12..16].try_into().unwrap()) as usize;

    let total_output_size = uncomp_size_of_comp_data + UNCOMPRESSED_DATA_SIZE;

    if output.capacity() < total_output_size {
        output.reserve(total_output_size);
    }
    output.resize(total_output_size, 0);

    let header_src_start = uncomp_header_offset + 0x10;

    if header_src_start + UNCOMPRESSED_DATA_SIZE > input.len() {
        return None; // Out of bounds safety
    }

    // Copy the header to the start of the output
    output[0..UNCOMPRESSED_DATA_SIZE]
        .copy_from_slice(&input[header_src_start..header_src_start + UNCOMPRESSED_DATA_SIZE]);

    let mut reader = ReverseBitReader::new(input, header_src_start);

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