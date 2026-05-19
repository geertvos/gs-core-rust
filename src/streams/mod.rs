use std::io::{Read, Write};

#[derive(Debug)]
pub struct RandomAccessByteStream {
    blocksize: usize,
    buffers: Vec<Vec<u8>>,
    size: usize,
    pointer: usize,
}

impl Clone for RandomAccessByteStream {
    fn clone(&self) -> Self {
        RandomAccessByteStream {
            blocksize: self.blocksize,
            buffers: self.buffers.clone(),
            size: self.size,
            pointer: 0,
        }
    }
}

impl RandomAccessByteStream {
    pub fn new() -> Self {
        Self::with_blocksize(8000)
    }

    pub fn with_blocksize(blocksize: usize) -> Self {
        Self::with_blocksize_and_blocks(blocksize, 1)
    }

    pub fn with_blocksize_and_blocks(blocksize: usize, numblocks: usize) -> Self {
        assert!(blocksize >= 1, "Blocksize must be 1 or higher");
        assert!(numblocks >= 1, "Numblocks must be 1 or higher");
        let mut buffers = Vec::with_capacity(numblocks);
        for _ in 0..numblocks {
            buffers.push(vec![0u8; blocksize]);
        }
        RandomAccessByteStream {
            blocksize,
            buffers,
            size: 0,
            pointer: 0,
        }
    }

    pub fn write_byte(&mut self, b: u8) {
        let buffer_number = self.pointer / self.blocksize;
        let local_pointer = self.pointer - (buffer_number * self.blocksize);
        while self.buffers.len() <= buffer_number {
            self.buffers.push(vec![0u8; self.blocksize]);
        }
        self.buffers[buffer_number][local_pointer] = b;
        self.pointer += 1;
        if self.pointer > self.size {
            self.size = self.pointer;
        }
    }

    pub fn write_bytes(&mut self, buf: &[u8]) {
        let mut bp = 0;
        while bp < buf.len() {
            let buffer_number = self.pointer / self.blocksize;
            while self.buffers.len() <= buffer_number {
                self.buffers.push(vec![0u8; self.blocksize]);
            }
            let local_pointer = self.pointer - (buffer_number * self.blocksize);
            let remaining_in_block = self.blocksize - local_pointer;
            let remaining_in_buf = buf.len() - bp;
            let copy = remaining_in_block.min(remaining_in_buf);
            for x in 0..copy {
                self.buffers[buffer_number][local_pointer + x] = buf[bp + x];
            }
            self.pointer += copy;
            bp += copy;
        }
        if self.pointer > self.size {
            self.size = self.pointer;
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        let buffer_number = self.pointer / self.blocksize;
        let local_pointer = self.pointer - (buffer_number * self.blocksize);
        let data = self.buffers[buffer_number][local_pointer];
        self.pointer += 1;
        data
    }

    pub fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut data = vec![0u8; len];
        let mut read_count = 0;
        let mut to_read = len;
        while to_read > 0 {
            let buffer_number = self.pointer / self.blocksize;
            let local_pointer = self.pointer - (buffer_number * self.blocksize);
            if to_read <= self.blocksize - local_pointer {
                let buffer = &self.buffers[buffer_number];
                for i in 0..to_read {
                    data[read_count] = buffer[local_pointer + i];
                    self.pointer += 1;
                    read_count += 1;
                }
            } else {
                let remaining_in_block = self.blocksize - local_pointer;
                let buffer = &self.buffers[buffer_number];
                for i in 0..remaining_in_block {
                    data[read_count] = buffer[local_pointer + i];
                    self.pointer += 1;
                    read_count += 1;
                }
            }
            to_read = len - read_count;
        }
        data
    }

    pub fn write_int(&mut self, val: i32) {
        self.write_byte((val >> 0) as u8);
        self.write_byte((val >> 8) as u8);
        self.write_byte((val >> 16) as u8);
        self.write_byte((val >> 24) as u8);
    }

    pub fn read_int(&mut self) -> i32 {
        let i1 = self.read_byte();
        let i2 = self.read_byte();
        let i3 = self.read_byte();
        let i4 = self.read_byte();
        ((i1 as i32) & 0xFF)
            | (((i2 as i32) & 0xFF) << 8)
            | (((i3 as i32) & 0xFF) << 16)
            | (((i4 as i32)) << 24)
    }

    pub fn write_double(&mut self, val: f64) {
        let j = val.to_bits();
        self.write_byte((j >> 0) as u8);
        self.write_byte((j >> 8) as u8);
        self.write_byte((j >> 16) as u8);
        self.write_byte((j >> 24) as u8);
        self.write_byte((j >> 32) as u8);
        self.write_byte((j >> 40) as u8);
        self.write_byte((j >> 48) as u8);
        self.write_byte((j >> 56) as u8);
    }

    pub fn read_double(&mut self) -> f64 {
        let d1 = self.read_byte();
        let d2 = self.read_byte();
        let d3 = self.read_byte();
        let d4 = self.read_byte();
        let d5 = self.read_byte();
        let d6 = self.read_byte();
        let d7 = self.read_byte();
        let d8 = self.read_byte();
        let j = ((d1 as u64) & 0xFF)
            | (((d2 as u64) & 0xFF) << 8)
            | (((d3 as u64) & 0xFF) << 16)
            | (((d4 as u64) & 0xFF) << 24)
            | (((d5 as u64) & 0xFF) << 32)
            | (((d6 as u64) & 0xFF) << 40)
            | (((d7 as u64) & 0xFF) << 48)
            | ((d8 as u64) << 56);
        f64::from_bits(j)
    }

    pub fn write_bool(&mut self, val: bool) {
        self.write_int(if val { 1 } else { 0 });
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_byte() != 0
    }

    pub fn write_string(&mut self, val: &str) {
        let bytes: Vec<u8> = val.encode_utf16().flat_map(|c| c.to_be_bytes()).collect();
        // Prepend UTF-16 BOM like Java's getBytes("UTF-16")
        let mut with_bom = Vec::with_capacity(bytes.len() + 2);
        with_bom.push(0xFE);
        with_bom.push(0xFF);
        with_bom.extend_from_slice(&bytes);
        self.write_int(with_bom.len() as i32);
        self.write_bytes(&with_bom);
    }

    pub fn read_string(&mut self) -> String {
        let size = self.read_int() as usize;
        let bytes = self.read_bytes(size);
        // Decode UTF-16 (with BOM)
        let u16s: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        // Skip BOM if present
        let start = if !u16s.is_empty() && u16s[0] == 0xFEFF { 1 } else { 0 };
        String::from_utf16_lossy(&u16s[start..])
    }

    pub fn write_char(&mut self, val: char) {
        let v = val as u16;
        self.write_byte((v >> 0) as u8);
        self.write_byte((v >> 8) as u8);
    }

    pub fn read_char(&mut self) -> char {
        let b1 = self.read_byte();
        let b2 = self.read_byte();
        let val = ((b1 as u16) & 0xFF) | (((b2 as u16)) << 8);
        char::from_u32(val as u32).unwrap_or('\0')
    }

    pub fn seek(&mut self, pos: i32) {
        let pos = pos as usize;
        let buffer_number = pos / self.blocksize;
        if self.buffers.len() <= buffer_number {
            panic!("Index out of bounds");
        }
        self.pointer = pos;
    }

    pub fn get_pointer_position(&self) -> i32 {
        self.pointer as i32
    }

    pub fn size(&self) -> i32 {
        self.size as i32
    }

    pub fn get_bytes(&mut self) -> Vec<u8> {
        let pos = self.get_pointer_position();
        self.seek(0);
        let mut data = Vec::with_capacity(self.size);
        for _ in 0..self.size {
            data.push(self.read_byte());
        }
        self.seek(pos);
        data
    }

    pub fn set(&mut self, pos: i32, value: i32) {
        let old_pos = self.get_pointer_position();
        self.seek(pos);
        self.write_int(value);
        self.seek(old_pos);
    }

    pub fn write_to(&mut self, stream: &mut dyn Write) -> std::io::Result<()> {
        let mut to_write = self.size;
        let mut current_block = 0;
        while to_write > 0 {
            if to_write > self.blocksize {
                let buf = &self.buffers[current_block];
                stream.write_all(buf)?;
                to_write -= buf.len();
                current_block += 1;
            } else {
                let buf = &self.buffers[current_block];
                stream.write_all(&buf[..to_write])?;
                to_write = 0;
            }
        }
        stream.flush()
    }

    pub fn read_from(&mut self, stream: &mut dyn Read) -> std::io::Result<()> {
        self.buffers.clear();
        self.pointer = 0;
        self.size = 0;
        loop {
            let mut buf = vec![0u8; self.blocksize];
            let mut filled = 0;
            while filled < self.blocksize {
                match stream.read(&mut buf[filled..]) {
                    Ok(0) => break,
                    Ok(n) => filled += n,
                    Err(e) => return Err(e),
                }
            }
            if filled == 0 {
                break;
            }
            self.buffers.push(buf);
            self.size += filled;
            if filled < self.blocksize {
                break;
            }
        }
        Ok(())
    }
}

impl Default for RandomAccessByteStream {
    fn default() -> Self {
        Self::new()
    }
}
