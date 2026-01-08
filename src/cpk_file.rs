#[derive(Debug, Default)]
pub struct CpkFile {
    pub user_string: Option<String>,
    pub directory: Option<String>,
    pub file_name: String,
    pub file_offset: u64,
    pub file_size: u32,
    pub extract_size: u32,
}