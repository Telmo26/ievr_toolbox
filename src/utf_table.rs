use std::{
    array::TryFromSliceError,
};

use crate::DecryptedCpk;

pub const BASE_OFFSET: u32 = 0x08;

pub const COLUMN_OFFSET: u32 = 0x20;

#[derive(Debug)]
pub struct UTFTable {
    pub data: Vec<u8>,
    pub metadata: Metadata,
}

impl UTFTable {
    pub fn new(file: &DecryptedCpk, offset: usize) -> std::io::Result<UTFTable> {
        // Skip the CPK, TOC, ITOC... header and the unused fields

        // Read the 4-byte size field
        let size_buf: [u8; 4] = file[8 + offset..12 + offset].try_into().unwrap();
        let size = u32::from_le_bytes(size_buf) as usize;

        // Read the entire table
        let data = &file[16 + offset..16 + size + offset];

        if is_utf_encrypted(&data) {
            println!("Encrypted CPK")
        }

        let metadata = Metadata::new(&data);

        Ok(UTFTable {
            data: data.to_vec(),
            metadata,
        })
    }
}

#[derive(Debug)]
pub struct Metadata {
    pub rows_offset: u16,
    pub string_pool_offset: u32,
    pub _data_pool_offset: u32,
    pub column_count: u16,
    pub row_size_bytes: u16,
    pub row_count: u32,
}

impl Metadata {
    fn new(data: &[u8]) -> Metadata {
        let rows_offset = read_u16_be(&data, 0x0A).unwrap() + BASE_OFFSET as u16;
        let string_pool_offset = read_u32_be(&data, 0x0C).unwrap() + BASE_OFFSET;
        let _data_pool_offset = read_u32_be(&data, 0x10).unwrap() + BASE_OFFSET;
        let column_count = read_u16_be(&data, 0x18).unwrap();
        let row_size_bytes = read_u16_be(&data, 0x1A).unwrap();
        let row_count = read_u32_be(&data, 0x1C).unwrap();
        Metadata {
            rows_offset,
            string_pool_offset,
            _data_pool_offset,
            column_count,
            row_size_bytes,
            row_count,
        }
    }

    pub fn first_column_pos(&self) -> u32 {
        COLUMN_OFFSET
    }

    pub fn first_row_offset(&self) -> u32 {
        self.rows_offset as u32
    }
}

fn is_utf_encrypted(table: &[u8]) -> bool {
    if table.len() < 4 {
        return false;
    }
    let magic = u32::from_le_bytes([table[0], table[1], table[2], table[3]]);
    magic == 0xF5F39E1F
}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16, TryFromSliceError> {
    Ok(u16::from_be_bytes(data[offset..offset + 2].try_into()?))
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, TryFromSliceError> {
    Ok(u32::from_be_bytes(data[offset..offset + 4].try_into()?))
}
