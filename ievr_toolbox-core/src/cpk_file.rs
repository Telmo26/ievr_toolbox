use std::{cmp::Ordering, sync::Arc};

use crate::{CpkData, DecryptedCpk};

#[derive(Debug, Default)]
pub struct CpkFile {
    // CPK self Metadata
    pub user_string: Option<Arc<str>>,
    pub directory: Option<Arc<str>>,
    pub file_name: String,
    pub file_offset: u64,
    pub file_size: u32,
    pub extract_size: u32,

    data: Option<DecryptedCpk>,
}

impl CpkFile {
    pub fn set_decrypted_cpk(&mut self, decrypted_cpk: &DecryptedCpk) {
        self.data = Some(decrypted_cpk.clone());
    }

    pub fn compression_header(&self) -> Option<&[u8]> {
        if let Some(ref data) = self.data {
            Some(&data[self.file_offset as usize..self.file_offset as usize + 8])
        } else {
            None
        }
    }

    pub fn data(&self) -> Option<&[u8]> {
        if let Some(data) = &self.data {
            Some(&data[self.file_offset as usize..(self.file_offset as usize + self.file_size as usize)])
        } else {
            None
        }
    }

    pub fn last_cpk_file(&self) -> Option<bool> {
        if let Some(data) = &self.data {
            return Some(Arc::<CpkData>::strong_count(&data) == 1)
        }
        None
    }

    pub fn cpk_size(&self) -> Option<usize> {
        if let Some(data) = &self.data {
            return Some(data.len());
        }
        None
    }
}

/// All of this is to be able to do load balancing 
/// and file-size aware ordering of decompression
impl Ord for CpkFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.extract_size.cmp(&other.extract_size)
    }
}

impl PartialOrd for CpkFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CpkFile {
    fn eq(&self, other: &Self) -> bool {
        self.extract_size == other.extract_size
    }
}

impl Eq for CpkFile {}