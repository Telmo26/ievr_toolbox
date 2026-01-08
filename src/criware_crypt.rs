use std::{fs::File, path::Path, io::{Read, Seek, SeekFrom, Write}};

pub struct CriwareCrypt {
    input_file: File,
    keys: [u8; 4],
    crc32table: [u32; 256],
    
}

impl CriwareCrypt {
    pub fn new(path: &Path) -> Result<CriwareCrypt, std::io::Error>  {
        let input_file = File::open(path)?; 
        let filename = path.file_name().unwrap().to_str().unwrap();

        let crc32table = Self::initialize_table();
        let keys = Self::compute_key(filename, &crc32table);

        Ok(CriwareCrypt { 
            input_file,
            keys, 
            crc32table, 
        })   
    }

    pub fn decrypt(&mut self, output_file: &mut File) -> Result<(), std::io::Error>{
        let mut header = [0u8; 4];
        self.input_file.read_exact(&mut header)?;
        self.input_file.seek(SeekFrom::Start(0))?;

        // If already decrypted, just copy
        if &header == b"CPK " {
            std::io::copy(&mut self.input_file, output_file)?;
            return Ok(());
        }

        // Stream decrypt
        let mut buffer = vec![0u8; 1024 * 1024]; // 1 MB
        let mut offset: u64 = 0;

        loop {
            let bytes_read = self.input_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            self.decrypt_block(&mut buffer[..bytes_read], offset);
            output_file.write_all(&buffer[..bytes_read])?;

            offset += bytes_read as u64;
        }

        Ok(())
    }

    fn decrypt_block(&mut self, buffer: &mut [u8], file_offset: u64) {
        let mut current_crc = self.update_crc_state((file_offset & !3) as u32);
        for i in 0..buffer.len() {
            let global_pos = file_offset + i as u64;

            if (global_pos & 3) == 0 {
                current_crc = self.update_crc_state(global_pos as u32);
            }

            let base_shift = (global_pos & 3) << 1;
            
            let mut r8 = (current_crc >> (base_shift +8)) & 3;
            let mut rdx = (current_crc >> base_shift) & 0xFF;
            let mut mask = (rdx << 2) & 0xFF;
            r8 |= mask;

            rdx = (current_crc >> (base_shift + 16)) & 3;
            mask = (r8 << 2) & 0xFF;
            r8 = mask | rdx;

            rdx = (current_crc >> (base_shift + 24)) & 3;
            mask = (r8 << 2) & 0xFF;
            r8 = mask | rdx;

            buffer[i] ^= r8 as u8;
        }
    }

    fn initialize_table() -> [u32; 256] {
        let mut table: [u32; 256] = [0; 256];
        let polynomial = 0xEDB88320;

        for i in 0..256 {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ polynomial;
                } else {
                    crc >>= 1;
                }
            }
            table[i] = crc;
        }

        table
    }

    fn compute_key(filename: &str, table: &[u32; 256]) -> [u8; 4] {
        let mut crc: u32 = 0xFFFF_FFFF;

        for &b in filename.as_bytes() {
            let index = ((crc ^ b as u32) & 0xFF) as u32;
            crc = (crc >> 8) ^ table[index as usize];
        }

        (!crc).to_le_bytes()
    }

    fn update_crc_state(&mut self, seed: u32) -> u32 {
        let mut crc = !seed;

        for k in 0..4 {
            let mut index = (crc & 0xFF) as u8;
            index ^= self.keys[k];
            crc = (crc >> 8) ^ self.crc32table[index as usize];
        }

        !crc
    }
}