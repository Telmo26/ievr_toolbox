use std::sync::Arc;
use memmap2::Mmap;

#[derive(Debug, Default)]
pub struct CpkFile {
    // CPK self Metadata
    pub user_string: Option<Arc<str>>,
    pub directory: Option<Arc<str>>,
    pub file_name: String,
    pub file_offset: u64,
    pub file_size: u32,
    pub extract_size: u32,

    mmap: Option<Arc<Mmap>>,
}

impl CpkFile {
    pub fn set_decrypted_cpk(&mut self, decrypted_cpk: &Arc<Mmap>) {
        self.mmap = Some(decrypted_cpk.clone());
    }

    pub fn compression_header(&self) -> Option<&[u8]> {
        if let Some(ref data) = self.mmap {
            Some(&data[self.file_offset as usize..self.file_offset as usize + 8])
        } else {
            None
        }
    }

    pub fn data(&self) -> Option<&[u8]> {
        if let Some(ref data) = self.mmap {
            Some(&data[self.file_offset as usize..(self.file_offset as usize + self.file_size as usize)])
        } else {
            None
        }
    }
}