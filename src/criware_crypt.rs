use std::{fs::File, io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write}, path::Path};

const BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8 MB

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

        let file = self.input_file.try_clone().unwrap();

        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
        let mut writer = BufWriter::with_capacity(BUFFER_SIZE, output_file);

        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut offset: u64 = 0;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            self.decrypt_block(&mut buffer[..bytes_read], offset);
            writer.write_all(&buffer[..bytes_read])?;

            offset += bytes_read as u64;
        }

        Ok(())
    }

    pub fn decrypt_ram(&mut self) -> std::io::Result<Vec<u8>> {
        let mut reader = BufReader::with_capacity(BUFFER_SIZE, &self.input_file);

        let size = self.input_file.metadata()?.len() as usize;

        let mut buffer = Vec::with_capacity(size);
        reader.read_to_end(&mut buffer)?;

        // If already decrypted, just copy
        if &buffer[0..4] == b"CPK " {
            return Ok(buffer);
        }

        self.decrypt_block(&mut buffer, 0);
        Ok(buffer)

    }

    fn decrypt_block(&mut self, buffer: &mut [u8], file_offset: u64) {
        let (prefix, middle, suffix) = unsafe { buffer.align_to_mut::<u32>() };
        
        let mut current_pos = file_offset;

        // 2. Handle leading bytes (if any) to reach u32 alignment
        for byte in prefix {
            let crc = self.update_crc_state(current_pos as u32);
            let ks = key_stream(crc)[current_pos as usize % 4];
            *byte ^= ks;
            current_pos += 1;
        }

        // 3. HOT LOOP: Process 4 bytes at a time
        for chunk in middle {
            // We use the global position of the START of this 4-byte chunk
            let crc = self.update_crc_state(current_pos as u32);
            
            // Compute the entire 4-byte keystream as a u32
            let key = key_stream_u32(crc); // u32::from_le_bytes(key_stream(crc));
            
            // Single XOR operation for 4 bytes
            *chunk ^= key;
            
            current_pos += 4;
        }

        // 4. Handle trailing bytes (if any)
        for byte in suffix {
            let crc = self.update_crc_state(current_pos as u32);
            let ks = key_stream(crc)[current_pos as usize % 4];
            *byte ^= ks;
            current_pos += 1;
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

        let mut idx = ((crc & 0xFF) as u8) ^ self.keys[0];
        crc = (crc >> 8) ^ self.crc32table[idx as usize];

        idx = ((crc & 0xFF) as u8) ^ self.keys[1];
        crc = (crc >> 8) ^ self.crc32table[idx as usize];

        idx = ((crc & 0xFF) as u8) ^ self.keys[2];
        crc = (crc >> 8) ^ self.crc32table[idx as usize];

        idx = ((crc & 0xFF) as u8) ^ self.keys[3];
        crc = (crc >> 8) ^ self.crc32table[idx as usize];

        !crc
    }
}

fn key_stream(crc: u32) -> [u8; 4] {
    let mut keys = [0u8; 4];
    for line in 0..4 {
        let base_shift = line << 1;
        let mut r8 = (crc >> (base_shift + 8)) & 3;
        let mut rdx = (crc >> base_shift) & 0xFF;
        let mut mask = (rdx << 2) & 0xFF;
        r8 |= mask;

        rdx = (crc >> (base_shift + 16)) & 3;
        mask = (r8 << 2) & 0xFF;
        r8 = mask | rdx;

        rdx = (crc >> (base_shift + 24)) & 3;
        mask = (r8 << 2) & 0xFF;
        r8 = mask | rdx;

        keys[line] = r8 as u8;
    }
    keys    
}

#[inline(always)]
fn key_stream_u32(crc: u32) -> u32 {
    let mut final_ks: u32 = 0;

    for lane in 0..4 {
        let s = lane << 1;
        
        let mut r8 = (crc >> (s + 8)) & 3;
        
        // Original logic: rdx = (crc >> s) & 0xFF; r8 |= (rdx << 2) & 0xFF
        r8 = (r8 | (((crc >> s) & 0xFF) << 2)) & 0xFF;

        // Original logic: rdx = (crc >> (s + 16)) & 3; r8 = ((r8 << 2) & 0xFF) | rdx
        r8 = ((r8 << 2) & 0xFF) | ((crc >> (s + 16)) & 3);

        // Original logic: rdx = (crc >> (s + 24)) & 3; r8 = ((r8 << 2) & 0xFF) | rdx
        r8 = ((r8 << 2) & 0xFF) | ((crc >> (s + 24)) & 3);

        // Pack into the u32 so that lane 0 is the first byte in memory (LE)
        final_ks |= (r8 as u32) << (lane * 8);
    }

    final_ks
}
