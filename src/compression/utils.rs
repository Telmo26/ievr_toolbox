use std::{fs::File, io::{Read, Seek}, path::{Path, PathBuf}};

pub fn is_compressed(file: &mut File) -> std::io::Result<bool> {
    // We read from the start of the file
    file.seek(std::io::SeekFrom::Start(0))?;

    let mut buffer = [0u8; 8];
    match file.read_exact(&mut buffer) {
        Ok(_) => {
            // Read successful, check the magic bytes
            Ok(&buffer == b"CRILAYLA")
        }
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            // File is smaller than 8 bytes, so it cannot be compressed
            Ok(false)
        }
        Err(e) => {
            // Some other legitimate error (permission denied, drive disconnected, etc.)
            Err(e)
        }
    }
}

pub fn replace_prefix(original: &Path, new_prefix: &Path) -> PathBuf {
    let components: Vec<_> = original.components().collect();
    
    // Find the position of the "data" folder component
    if let Some(pos) = components.iter().position(|c| c.as_os_str() == "data") {
        let mut new_path = new_prefix.to_path_buf();
        
        // Append "data" and everything after it
        for i in pos..components.len() {
            new_path.push(components[i]);
        }
        new_path
    } else {
        original.to_path_buf()
    }
}