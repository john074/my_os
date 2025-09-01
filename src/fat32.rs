use core::ptr::copy_nonoverlapping;
use x86::io::{inb, inw, outb, outw};
use xmas_elf::ElfFile;
use xmas_elf::program::Type;

use crate::alloc::string::ToString;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::multitasking;
use crate::println;


pub static mut FS_PTR: *mut FAT32Volume = core::ptr::null_mut();
pub static mut RETURN_ADDR: usize = 0;

pub fn load_elf_and_jump(elf_data: &[u8]) {
    let elf = ElfFile::new(elf_data).expect("Invalid ELF");

    for ph in elf.program_iter() {
        if ph.get_type().unwrap() != Type::Load {
            continue;
        }

        let p_offset = ph.offset() as usize;
        let p_vaddr = ph.virtual_addr() as usize;
        let p_filesz = ph.file_size() as usize;
        let p_memsz = ph.mem_size() as usize;

        unsafe {
            let src = elf_data.as_ptr().add(p_offset);
            let dst = p_vaddr as *mut u8;
            copy_nonoverlapping(src, dst, p_filesz);

            core::ptr::write_bytes(dst.add(p_filesz), 0, p_memsz - p_filesz);
        }
    }

    let entry = elf.header.pt2.entry_point() as usize;
    unsafe {
        RETURN_ADDR = return_to_kernel as usize;
        core::arch::asm!(
            "mov rax, {0}", // entry point
            "jmp rax",
            in(reg) entry,
            options(noreturn)
        );
    }
}

extern "C" fn return_to_kernel() {
    let executor = unsafe { &mut *multitasking::EXECUTOR_PTR };
    executor.run();
}

pub trait BlockDevice {
    fn read_sector(&mut self, lba: u32, buf: &mut [u8; 512]);
    fn write_sector(&mut self, lba: u32, buf: &[u8; 512]);
}

pub unsafe fn identify_drive() {
    unsafe {
        outb(0x1F6, 0xA0);
        outb(0x1F7, 0xEC); // IDENTIFY

        if inb(0x1F7) == 0 {
            println!("No drive found");
            return;
        }
        while (inb(0x1F7) & 0x80) != 0 {}
        while (inb(0x1F7) & 0x08) == 0 {}
        for _ in 0..256 {
            let _ = inw(0x1F0);
        }

        println!("Drive identified!");
    }
}

// ATA ports
// 0x1F0: Data Register (read/write)
// 0x1F1: Error Register
// 0x1F2: Sector Count
// 0x1F3: LBA Low
// 0x1F4: LBA Mid
// 0x1F5: LBA High
// 0x1F6: Device/Head
// 0x1F7: Status/Command

// DF - Device fault
// DRQ - Data request

#[derive(Copy, Clone)]
pub struct AtaDevice;

impl AtaDevice {
    pub fn new() -> Self {
        AtaDevice
    }

    unsafe fn wait_ready() {
        unsafe {
            while (inb(0x1F7) & 0x80) != 0 {} // BSY

            let status = inb(0x1F7);
            if (status & 0x01) != 0 || (status & 0x20) != 0 { // ERR / DF
                let error = inb(0x1F1);
                panic!("ATA Error: Status={:#x}, Error={:#x}", status, error);
            }

            let mut timeout = 100_000;
            while (inb(0x1F7) & 0x08) == 0 { // DRQ
                timeout -= 1;
                if timeout == 0 {
                    println!("Timeout on Secondary IDE");
                    let status = inb(0x1F7);
                    println!("Status(0x1F7): {:#x}", status);
                    let error = inb(0x1F1);
                    println!("Error(0x1F1): {:#x}", error);
                    let statusf6 = inb(0x1F6);
                    println!("status(0x1F6): {:#x}", statusf6);
                }
            }
        }
    }
}

impl BlockDevice for AtaDevice {
    fn read_sector(&mut self, lba: u32, buf: &mut[u8; 512]) {
        unsafe {
            outb(0x1F6, 0xF0 | ((lba >> 24) & 0x0F) as u8); // Primary slave
            outb(0x1F2, 1); 								// Sectors to read
            outb(0x1F3, (lba & 0xFF) as u8); 				// LBA 0–7
            outb(0x1F4, ((lba >> 8) & 0xFF) as u8); 		// LBA 8–15
            outb(0x1F5, ((lba >> 16) & 0xFF) as u8); 		// LBA 16–23
            outb(0x1F7, 0x20); 								// READ SECTORS
            Self::wait_ready();
            for i in 0..256 {								// Read 512 bytes (256 words) 
                let w = inw(0x1F0);
                buf[i * 2] = (w & 0xFF) as u8;
                buf[i * 2 + 1] = (w >> 8) as u8;
            }
        }
    }

    fn write_sector(&mut self, lba: u32, buf: &[u8; 512]) {
        unsafe {
            outb(0x1F6, 0xF0 | ((lba >> 24) & 0x0F) as u8); // Primary slave
            outb(0x1F2, 1); 								// 1 sector
            outb(0x1F3, (lba & 0xFF) as u8);
            outb(0x1F4, ((lba >> 8) & 0xFF) as u8);
            outb(0x1F5, ((lba >> 16) & 0xFF) as u8);
            outb(0x1F7, 0x30); 								// WRITE SECTOR command

            Self::wait_ready(); 							// DRQ

            for i in 0..256 {								// write 512 bytes (256 words)
                let lo = buf[i * 2] as u16;
                let hi = buf[i * 2 + 1] as u16;
                let word = lo | (hi << 8);
                outw(0x1F0, word);
            }

            while (inb(0x1F7) & 0x80) != 0 {}				// BSY
        }
    }
}

// File sttributes
// 0x01 - Read-only
// 0x02 - Hidden
// 0x04 - System
// 0x08 - Volume label
// 0x10 - Directory
// 0x20 - Archive

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: [u8; 11],
    pub attr: u8,
    pub cluster_high: u16,
    pub cluster_low: u16,
    pub file_size: u32,
}

impl DirectoryEntry {
    pub fn is_directory(&self) -> bool {
        self.attr & 0x10 != 0
    }

    pub fn starting_cluster(&self) -> u32 {
        ((self.cluster_high as u32) << 16) | self.cluster_low as u32
    }

    pub fn filename(&self) -> String {
        let raw = &self.name;
        let name = String::from_utf8_lossy(&raw[..8]).trim().to_string();
        let ext = String::from_utf8_lossy(&raw[8..]).trim().to_string();

        if ext.is_empty() {
            name
        } else {
            format!("{}.{}", name, ext)
        }
    }
}


pub struct FAT {
    pub fat_start_lba: u32,
    pub fat_size_sectors: u32,
    pub bytes_per_sector: u16,
}

impl FAT {
    pub fn new(fat_start_lba: u32, fat_size_sectors: u32, bytes_per_sector: u16) -> Self {
        FAT {
            fat_start_lba,
            fat_size_sectors,
            bytes_per_sector,
        }
    }

    fn read_entry(&self, device: &mut dyn BlockDevice, cluster: u32) -> u32 {
        let fat_offset = cluster * 4;
        let sector = fat_offset / self.bytes_per_sector as u32;
        let offset = fat_offset % self.bytes_per_sector as u32;

        let mut buf = [0u8; 512];
        device.read_sector(self.fat_start_lba + sector, &mut buf);
        u32::from_le_bytes([				// read 4 bytes as u32
            buf[offset as usize],
            buf[offset as usize + 1],
            buf[offset as usize + 2],
            buf[offset as usize + 3],
        ]) & 0x0FFFFFFF						// hide 4 high bytes (reserved)
    }

    fn write_entry(&self, device: &mut dyn BlockDevice, cluster: u32, value: u32) {
        let fat_offset = cluster * 4;
        let sector = fat_offset / self.bytes_per_sector as u32;
        let offset = fat_offset % self.bytes_per_sector as u32;

        let mut buf = [0u8; 512];
        device.read_sector(self.fat_start_lba + sector, &mut buf);

		// replace 4 bytes of read data
        buf[offset as usize..offset as usize + 4].copy_from_slice(&value.to_le_bytes());
        device.write_sector(self.fat_start_lba + sector, &buf);
    }
}

// 0x00000000 - free
// 0x00000001 - reserved
// 0x0FFFFFF7 - bad cluster
// 0x0FFFFFF8 - 0x0FFFFFFF - chain end


pub struct FAT32Volume {
    pub fat: FAT,
    pub cluster_size: usize,
    pub root_dir_cluster: u32,
    pub device: Box<dyn BlockDevice>,
    pub sectors_per_cluster: u8,
    pub bytes_per_sector: u16,
    pub reserved_sector_count: u16,
    pub fat_size_sectors: u32,
    pub num_fats: u8,
}

// dir structure (32 bytes)
// 0-10: name (8.3)
// 11: attrib
// 12: reserved
// 13: creation timr (tenth of a sec)
// 14-15: creation time
// 16-17: creation date
// 18-19: last access
// 20-21: high
// 22-23: last modification time
// 24-25: last modification date
// 26-27: low
// 28-32: size 
// 
// 0x00 - end
// 0xE5 - removed
// 0x0F - LFN

impl FAT32Volume {
    pub fn next_cluster(&mut self, current: u32) -> Option<u32> {
        let val = self.fat.read_entry(&mut *self.device, current);
        if val >= 0x0FFFFFF8 {		// last?
            None
        } else {
            Some(val)
        }
    }

    pub fn allocate_cluster(&mut self) -> Option<u32> {
        let entries_per_sector = self.bytes_per_sector as u32 / 4;
        let total_entries = self.fat_size_sectors * entries_per_sector;

        let mut buf = [0u8; 512];
        for i in 2..total_entries {
            let val = self.fat.read_entry(&mut *self.device, i);
            if val == 0 {		// if free
                self.fat.write_entry(&mut *self.device, i, 0x0FFFFFFF);
                return Some(i);  // cluster num
            }
        }
        None
    }

    pub fn set_next_cluster(&mut self, current: u32, next: u32) {
        self.fat.write_entry(&mut *self.device, current, next);
    }

    pub fn free_cluster_chain(&mut self, mut cluster: u32) {
        while cluster < 0x0FFFFFF8 {
            let next = self.fat.read_entry(&mut *self.device, cluster);
            self.fat.write_entry(&mut *self.device, cluster, 0); // set as free
            if next >= 0x0FFFFFF8 {
                break;
            }
            cluster = next;
        }
    }

    pub fn read_cluster(&mut self, cluster: u32) -> Vec<u8> {
        let start_lba = self.first_sector_of_cluster(cluster);
        let mut buf = vec![0u8; self.cluster_size];

        for i in 0..self.sectors_per_cluster {
            let mut sector = [0u8; 512];
            self.device.read_sector(start_lba + i as u32, &mut sector);
            let offset = i as usize * self.bytes_per_sector as usize;
            buf[offset..offset + 512].copy_from_slice(&sector);
        }
        buf
    }

    pub fn write_cluster(&mut self, cluster: u32, data: &[u8]) {
        let start_lba = self.first_sector_of_cluster(cluster);
        for i in 0..self.sectors_per_cluster {
            let sector_offset = i as usize * self.bytes_per_sector as usize;
            let mut sector_data = [0u8; 512];
            sector_data.copy_from_slice(&data[sector_offset..sector_offset + 512]);
            self.device.write_sector(start_lba + i as u32, &sector_data);
        }
    }

    fn first_sector_of_cluster(&self, cluster: u32) -> u32 {
        let data_start = self.reserved_sector_count as u32 + (self.num_fats as u32 * self.fat_size_sectors);
        data_start + (cluster - 2) * self.sectors_per_cluster as u32
    }

    pub fn set_entry_cluster(&mut self, path: &str, cluster: u32) -> Result<(), &'static str> {
        let (dir_path, filename) = split_path(path)?;
        let cluster_dir = self.find_directory_cluster(dir_path)?;
        let mut entries = self.read_directory(cluster_dir);
    
        for entry in entries.iter_mut() {
            if entry.filename().eq_ignore_ascii_case(filename) {
                entry.cluster_low = (cluster & 0xFFFF) as u16;
                entry.cluster_high = ((cluster >> 16) & 0xFFFF) as u16;
    
                self.write_directory(cluster_dir, &entries)?;
                return Ok(());
            }
        }
    
        Err("File not found in the directory")
    }

    pub fn find_directory_cluster(&mut self, path: &str) -> Result<u32, &'static str> {
        if path == "/" {
            return Ok(self.root_dir_cluster);
        }
    
        let mut components = path.trim_matches('/').split('/').peekable();
        let mut cluster = self.root_dir_cluster;
    
        while let Some(component) = components.next() {
            let entries = self.read_directory(cluster);
            let mut found = false;
    
            for entry in entries {
                if entry.filename().eq_ignore_ascii_case(component) && entry.is_directory() {
                    cluster = entry.starting_cluster();
                    found = true;
                    break;
                }
            }
    
            if !found {
                return Err("Directory not found");
            }
        }
    
        Ok(cluster)
    }
    
    pub fn write_directory(&mut self, cluster: u32, entries: &[DirectoryEntry]) -> Result<(), &'static str> {
        let mut data = vec![0u8; self.cluster_size];
        let mut offset = 0;
    
        for entry in entries {
            if offset + 32 > data.len() {
                return Err("Too many entries in directory");
            }
    
            data[offset..offset + 11].copy_from_slice(&entry.name); 								// name 
            data[offset + 11] = entry.attr; 														// attib	
            data[offset + 20..offset + 22].copy_from_slice(&entry.cluster_high.to_le_bytes());		// high
            data[offset + 26..offset + 28].copy_from_slice(&entry.cluster_low.to_le_bytes());		// low
            data[offset + 28..offset + 32].copy_from_slice(&entry.file_size.to_le_bytes());			// size
    
            offset += 32;
        }
    
        self.write_cluster(cluster, &data);
        Ok(())
    }
    

    pub fn read_directory(&mut self, cluster: u32) -> Vec<DirectoryEntry> {
        let mut entries = Vec::new();
        let mut current_cluster = cluster;
        while current_cluster < 0x0FFFFFF8 {
            let data = self.read_cluster(current_cluster);
            for i in 0..(self.cluster_size / 32) {		// 32 bytes per entry
                let offset = i * 32;
                let entry = &data[offset..offset + 32];
                if entry[0] == 0x00 { return entries; }
                if entry[0] == 0xE5 || entry[11] == 0x0F { continue; }
                let dir_entry = DirectoryEntry {
                    name: entry[0..11].try_into().unwrap(),
                    attr: entry[11],
                    cluster_high: u16::from_le_bytes([entry[20], entry[21]]),
                    cluster_low: u16::from_le_bytes([entry[26], entry[27]]),
                    file_size: u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]),
                };
                entries.push(dir_entry);
            }
            match self.next_cluster(current_cluster) {
                Some(next) => current_cluster = next,
                None => break,
            }
        }
        entries
    }

    pub fn find_path(&mut self, path: &str) -> Option<DirectoryEntry> {
        let mut components = path.trim_matches('/').split('/').peekable();
        let mut cluster = self.root_dir_cluster;
        while let Some(component) = components.next() {
            let entries = self.read_directory(cluster);
            let mut found = false;
            for entry in entries {
                if entry.filename().eq_ignore_ascii_case(component) {
                    if components.peek().is_some() {
                        if entry.is_directory() {
                            cluster = entry.starting_cluster();
                            found = true;
                            break;
                        } else {
                            return None;
                        }
                    } else {
                        return Some(entry);
                    }
                }
            }
            if !found { return None; }
        }
        None
    }

    pub fn write_directory_entry(&mut self, cluster: u32, entry: &DirectoryEntry) -> Result<(), &'static str> {
        let mut current_cluster = cluster;

        while current_cluster < 0x0FFFFFF8 {
            let mut data = self.read_cluster(current_cluster);

            for i in 0..(self.cluster_size / 32) {
                let offset = i * 32;
                if data[offset] == 0x00 || data[offset] == 0xE5 {
                    data[offset..offset + 11].copy_from_slice(&entry.name);									// name
                    data[offset + 11] = entry.attr;															// attr 
                    data[offset + 20..offset + 22].copy_from_slice(&entry.cluster_high.to_le_bytes());		// high
                    data[offset + 26..offset + 28].copy_from_slice(&entry.cluster_low.to_le_bytes());		// low
                    data[offset + 28..offset + 32].copy_from_slice(&entry.file_size.to_le_bytes());			// size

                    self.write_cluster(current_cluster, &data);
                    return Ok(());
                }
            }

            match self.next_cluster(current_cluster) {
                Some(next) => current_cluster = next,
                None => break,
            }
        }

        Err("Insufficient space for creating DirectoryEntry")
    }

	pub fn create_directory(&mut self, path: &str) -> Result<(), &'static str> {
        let path = path.trim_matches('/');
    
        let (parent_path, name) = match path.rfind('/') {
            Some(pos) => (&path[..pos], &path[pos + 1..]),
            None => ("", path),
        };
    
        let parent_cluster = if parent_path.is_empty() {
            self.root_dir_cluster
        } else {
            self.find_path(parent_path)
                .ok_or("Parent dir not found")?
                .starting_cluster()
        };
    
        let new_cluster = self.allocate_cluster().ok_or("No clusters available")?;
    
        let dir_data: &mut [u8] = &mut [0u8; 4096];
    
        if self.cluster_size > 4096 {
            return Err("Cluster is too large: increase size of the dir_data buffer");
        }
    
        // "." and ".."
        write_dot_entry(&mut dir_data[0..32], ".", 0x10, new_cluster);
        write_dot_entry(&mut dir_data[32..64], "..", 0x10, parent_cluster);
        self.write_cluster(new_cluster, &dir_data[..self.cluster_size]);

        let name_raw = to_short_name(name);
    
        let entry = DirectoryEntry {
            name: name_raw,
            attr: 0x10, // directory
            cluster_high: (new_cluster >> 16) as u16,
            cluster_low: new_cluster as u16,
            file_size: 0,
        };
    
        self.write_directory_entry(parent_cluster, &entry)?;
        Ok(())
    }
    


	pub fn create_file(&mut self, path: &str, size: u32) -> Result<(), &'static str> {
        let path = path.trim_matches('/');

        let (parent_path, name) = match path.rfind('/') {
            Some(pos) => (&path[..pos], &path[pos + 1..]),
            None => ("", path),
        };

        let parent_cluster = if parent_path.is_empty() {
            self.root_dir_cluster
        } else {
            self.find_path(parent_path)
                .ok_or("Parent dir not founf")?
                .starting_cluster()
        };

        let clusters_needed = ((size + self.cluster_size as u32 - 1) / self.cluster_size as u32) as usize;

        let mut cluster_chain = Vec::new();
        for _ in 0..clusters_needed {
            let cluster = self.allocate_cluster().ok_or("No clusters available")?;
            if let Some(&prev) = cluster_chain.last() {
                self.set_next_cluster(prev, cluster);
            }
            cluster_chain.push(cluster);
        }

        if let Some(&last) = cluster_chain.last() {
            self.set_next_cluster(last, 0x0FFFFFFF); // EOF
        }

        // DirectoryEntry
        let name_raw = to_short_name(name);

        let start_cluster = *cluster_chain.first().unwrap_or(&0);

        let entry = DirectoryEntry {
            name: name_raw,
            attr: 0x20,
            cluster_high: (start_cluster >> 16) as u16,
            cluster_low: start_cluster as u16,
            file_size: size,
        };

        self.write_directory_entry(parent_cluster, &entry)?;

        Ok(())
    }


    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, &'static str> {
        let entry = self.find_path(path).ok_or("File not found")?;

        if entry.is_directory() {
            return Err("Directory, not a file");
        }

        let mut remaining = entry.file_size;
        let mut cluster = entry.starting_cluster();
        let mut result = Vec::with_capacity(remaining as usize);

        while cluster < 0x0FFFFFF8 && remaining > 0 {
            let data = self.read_cluster(cluster);
            let to_copy = remaining.min(self.cluster_size as u32);
            result.extend_from_slice(&data[..to_copy as usize]);
            remaining -= to_copy;

            match self.next_cluster(cluster) {
                Some(next) => cluster = next,
                None => break,
            }
        }

        Ok(result)
    }

	pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), &'static str> {
        let entry = self.find_path(path).ok_or("File not found")?;
        if entry.is_directory() {
            return Err("Cant write in directory");
        }
    
        let mut current_cluster = entry.starting_cluster();
        
        if current_cluster < 2 {
            current_cluster = self.allocate_cluster().ok_or("No clusters available")?;
            self.set_next_cluster(current_cluster, 0x0FFFFFFF);
        
            self.set_entry_cluster(path, current_cluster)?;
        }
        
        let mut offset = 0;
        let total_size = data.len();
    
        let mut cluster_chain_buf = [0u32; 128];
        let mut chain_len = 0;
    
        cluster_chain_buf[chain_len] = current_cluster;
        chain_len += 1;
    
        while let Some(next) = self.next_cluster(current_cluster) {
            if next >= 0x0FFFFFF8 {
                break;
            }
            current_cluster = next;
            if chain_len >= cluster_chain_buf.len() {
                return Err("Clusters chain is too long");
            }
            cluster_chain_buf[chain_len] = current_cluster;
            chain_len += 1;
        }
    
        let clusters_needed = (total_size + self.cluster_size - 1) / self.cluster_size;
    
        while chain_len < clusters_needed {
            let new_cluster = self.allocate_cluster().ok_or("No clusters available")?;
            self.set_next_cluster(cluster_chain_buf[chain_len - 1], new_cluster);
            cluster_chain_buf[chain_len] = new_cluster;
            self.set_next_cluster(new_cluster, 0x0FFFFFFF);
            chain_len += 1;
        }
    
        let mut block = [0u8; 4096];
        if self.cluster_size > 4096 {
            return Err("Cluster is too large: increase size of the block buffer");
        }
    
        offset = 0;
        for i in 0..clusters_needed {
            let cluster = cluster_chain_buf[i];
            let to_copy = core::cmp::min(total_size - offset, self.cluster_size);
            for j in 0..to_copy {
                block[j] = data[offset + j];
            }
            self.write_cluster(cluster, &block[..self.cluster_size]);
            offset += to_copy;
        }
    
		if chain_len > clusters_needed {
		    let to_free = &cluster_chain_buf[clusters_needed..chain_len];
		    self.set_next_cluster(cluster_chain_buf[clusters_needed - 1], 0x0FFFFFFF);
		    for &c in to_free {
		        self.free_cluster_chain(c);
		    }
		}
    
        self.update_file_size(path, total_size as u32)?;
        Ok(())
    }
    

	pub fn delete_file(&mut self, path: &str) -> Result<(), &'static str> {
        let path = path.trim_matches('/');
    
        let (parent_path, name) = match path.rfind('/') {
            Some(pos) => (&path[..pos], &path[pos + 1..]),
            None => ("", path),
        };
    
        let parent_cluster = if parent_path.is_empty() {
            self.root_dir_cluster
        } else {
            self.find_path(parent_path)
                .ok_or("Parent dir not found")?
                .starting_cluster()
        };
    
        self.delete_entry_from_dir(parent_cluster, name)
    }
    

	pub fn delete_directory(&mut self, path: &str) -> Result<(), &'static str> {
	    let entry = self.find_path(path).ok_or("Directory not found")?;
	    if !entry.is_directory() {
	        return Err("Not a directory");
	    }

	    let cluster = entry.starting_cluster();
	    let entries = self.read_directory(cluster);

	    let user_entries: Vec<_> = entries
	        .iter()
	        .filter(|e| e.filename() != "." && e.filename() != "..")
	        .collect();

	    if !user_entries.is_empty() {
	        return Err("Directory is not empty");
	    }

	    let (parent_path, name) = match path.rfind('/') {
	        Some(pos) => (&path[..pos], &path[pos + 1..]),
	        None => ("", path),
	    };

	    let parent_cluster = if parent_path.is_empty() {
	        self.root_dir_cluster
	    } else {
	        self.find_path(parent_path)
	            .ok_or("Parent dir not found")?
	            .starting_cluster()
	    };

	    self.delete_entry_from_dir(parent_cluster, name)
	}

	pub fn delete_entry_from_dir(&mut self, parent_cluster: u32, name: &str,) -> Result<(), &'static str> {
	    let raw_name = to_short_name(name);
	    let mut current_cluster = parent_cluster;

	    while current_cluster < 0x0FFFFFF8 {
	        let mut data = self.read_cluster(current_cluster);

	        for i in 0..(self.cluster_size / 32) {
	            let offset = i * 32;
	            let entry = &mut data[offset..offset + 32];

	            if entry[0] == 0x00 || entry[0] == 0xE5 || entry[11] == 0x0F {
	                continue;
	            }

	            if &entry[0..11] == &raw_name {
	                let cluster_low = u16::from_le_bytes([entry[26], entry[27]]);
	                let cluster_high = u16::from_le_bytes([entry[20], entry[21]]);
	                let first_cluster = ((cluster_high as u32) << 16) | (cluster_low as u32);
	                self.free_cluster_chain(first_cluster);

	                // delete
	                entry[0] = 0xE5;
	                self.write_cluster(current_cluster, &data);
	                return Ok(());
	            }
	        }

	        match self.next_cluster(current_cluster) {
	            Some(next) => current_cluster = next,
	            None => break,
	        }
	    }

	    Err("Entry not found")
	}


	pub fn update_file_size(&mut self, path: &str, new_size: u32) -> Result<(), &'static str> {
        let path = path.trim_matches('/');
    
        let (parent_path, name) = match path.rfind('/') {
            Some(pos) => (&path[..pos], &path[pos + 1..]),
            None => ("", path),
        };
        
        let parent_cluster = if parent_path.is_empty() {
            self.root_dir_cluster
        } else {
            self.find_path(parent_path)
                .ok_or("Parent dir not found")?
                .starting_cluster()
        };
    
        let mut current_cluster = parent_cluster;
    
        while current_cluster < 0x0FFFFFF8 {
            let mut data = self.read_cluster(current_cluster);
    
            for i in 0..(self.cluster_size / 32) {
                let offset = i * 32;
                let entry = &mut data[offset..offset + 32];
    
                if entry[0] == 0x00 || entry[0] == 0xE5 || entry[11] == 0x0F {
                    continue;
                }
    
                let raw_name = to_short_name(name);
    
                if &entry[0..11] == &raw_name {
                    entry[28..32].copy_from_slice(&new_size.to_le_bytes());
                    self.write_cluster(current_cluster, &data);
                    return Ok(());
                }
            }
    
            match self.next_cluster(current_cluster) {
                Some(next) => current_cluster = next,
                None => break,
            }
        }
    
        Err("DirectoryEntry not found")
    }
    
    
    pub fn list_dir(&mut self, path: &str) -> Result<Vec<String>, &'static str> {
        let cluster = if path == "/" {
            self.root_dir_cluster
        } else {
            let entry = self.find_path(path).ok_or("Directory not found")?;
            if !entry.is_directory() {
                return Err("Not a directory");
            }
            entry.starting_cluster()
        };

        let entries = self.read_directory(cluster);
        Ok(entries
            .into_iter()
            .filter(|e| e.filename() != "." && e.filename() != "..")
            .map(|e| {
                if e.is_directory() {
                    format!("{}/", e.filename())
                } else {
                    e.filename()
                }
            })
            .collect())
    }

    pub fn file_exists(&mut self, path: &str) -> bool {
        self.find_path(path).is_some()
    }
}

fn split_path(path: &str) -> Result<(&str, &str), &'static str> {
    let trimmed = path.trim_matches('/');
    if let Some(pos) = trimmed.rfind('/') {
        let (dir, file) = trimmed.split_at(pos);
        Ok((dir, &file[1..]))
    } else {
        Ok(("/", trimmed))
    }
}


fn write_dot_entry(buf: &mut [u8], name: &str, attr: u8, cluster: u32) {
    let mut raw_name = [b' '; 11];
    let name_bytes = name.as_bytes();
    for (i, b) in name_bytes.iter().take(11).enumerate() {
        raw_name[i] = *b;
    }

    buf[0..11].copy_from_slice(&raw_name);
    buf[11] = attr;
    buf[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
    buf[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());
    buf[28..32].copy_from_slice(&0u32.to_le_bytes());
}

fn to_short_name(name: &str) -> [u8; 11] {
    let mut short_name = [b' '; 11];
    let parts: Vec<&str> = name.split('.').collect();

    if parts.len() == 2 {
        for (i, b) in parts[0].bytes().take(8).enumerate() {
            short_name[i] = b.to_ascii_uppercase();
        }
        for (i, b) in parts[1].bytes().take(3).enumerate() {
            short_name[8 + i] = b.to_ascii_uppercase();
        }
    } else {
        for (i, b) in name.bytes().take(11).enumerate() {
            short_name[i] = b.to_ascii_uppercase();
        }
    }

    short_name
}


pub fn mount_fat32(mut device: Box<dyn BlockDevice>) -> Result<FAT32Volume, &'static str> {
    println!("Mounting File System...");

    let mut vbr = [0u8; 512];
    device.read_sector(0, &mut vbr);

    let bytes_per_sector = u16::from_le_bytes([vbr[11], vbr[12]]);
    let sectors_per_cluster = vbr[13];
    let reserved_sector_count = u16::from_le_bytes([vbr[14], vbr[15]]);
    let num_fats = vbr[16];
    let fat_size_sectors = u32::from_le_bytes([vbr[36], vbr[37], vbr[38], vbr[39]]);
    let root_dir_cluster = u32::from_le_bytes([vbr[44], vbr[45], vbr[46], vbr[47]]);

    let fat_start_lba = reserved_sector_count as u32;

    let fat = FAT::new(fat_start_lba, fat_size_sectors, bytes_per_sector);

    println!("File System    [OK]");

    Ok(FAT32Volume {
        fat,
        cluster_size: bytes_per_sector as usize * sectors_per_cluster as usize,
        root_dir_cluster,
        device,
        sectors_per_cluster,
        bytes_per_sector,
        reserved_sector_count,
        fat_size_sectors,
        num_fats,
    })
}
