use crate::cpk_file::CpkFile;

pub fn is_compressed(file: &CpkFile) -> bool {
    // We read from the start of the file
    if let Some(data) = file.data() {
        if data.len() < 8 {
            return false
        }
        // println!("Header: {:?} Compressed string: {:?}", &data[..8], b"CRILAYLA");
        return &data[..8] == b"CRILAYLA"
    }
    false
}