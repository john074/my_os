use x86::io::{ inb, inw, outb, outw };
use xmas_elf::ElfFile;
use xmas_elf::program::Type;
use core::ptr::copy_nonoverlapping;

use crate::println;

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
    //println!("Jumping to entry point at {:#x}", entry);
    let entry_fn: extern "C" fn() -> ! = unsafe { core::mem::transmute(entry) };
    entry_fn()
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

pub struct AtaDevice;

impl AtaDevice {
    pub fn new() -> Self {
        AtaDevice
    }

    unsafe fn wait_ready() {
   		unsafe{
        	while (inb(0x1F7) & 0x80) != 0 {} // BSY

			let status = inb(0x1F7);
    		if (status & 0x01) != 0 || (status & 0x20) != 0 {
        		let error = inb(0x1F1);
        		panic!("ATA Error: Status={:#x}, Error={:#x}", status, error);
    		}
        
       	 	let mut timeout = 100_000;
    	    while (inb(0x1F7) & 0x08) == 0  {  // DRQ
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
    fn read_sector(&mut self, lba: u32, buf: &mut [u8; 512]) {
        unsafe {
            outb(0x1F6, 0xF0 | ((lba >> 24) & 0x0F) as u8); //Primary slave
            outb(0x1F2, 1);                                 // Sectors to read
            outb(0x1F3, (lba & 0xFF) as u8);                // LBA 0–7
            outb(0x1F4, ((lba >> 8) & 0xFF) as u8);         // LBA 8–15
            outb(0x1F5, ((lba >> 16) & 0xFF) as u8);        // LBA 16–23
            outb(0x1F7, 0x20);                              // READ SECTORS
            Self::wait_ready();
            for i in 0..256 {
                let w = inw(0x1F0);
                buf[i * 2] = (w & 0xFF) as u8;
                buf[i * 2 + 1] = (w >> 8) as u8;
            }
        }
    }

    fn write_sector(&mut self, lba: u32, buf: &[u8; 512]) {
    	unsafe {
    		outb(0x1F6, 0xF0 | ((lba >> 24) & 0x0F) as u8); // Primary slave
    	    outb(0x1F2, 1); // 1 sector
    	    outb(0x1F3, (lba & 0xFF) as u8);
    	    outb(0x1F4, ((lba >> 8) & 0xFF) as u8);
    	    outb(0x1F5, ((lba >> 16) & 0xFF) as u8);
    	    outb(0x1F7, 0x30); // WRITE SECTOR command

    	    Self::wait_ready(); // DRQ
    	
    	    for i in 0..256 {
    	        let lo = buf[i * 2] as u16;
    	        let hi = buf[i * 2 + 1] as u16;
    	        let word = lo | (hi << 8);
    	        outw(0x1F0, word);
    	    }
    	
    	    while (inb(0x1F7) & 0x80) != 0 {}
      	}        
	}
}

pub struct Fat12Fs<D: BlockDevice> {
    device: D,
    fat_start: u32,
    data_start: u32,
    root_dir_start: u32,
}

impl<D: BlockDevice> Fat12Fs<D> {
    pub fn new(mut device: D) -> Self {
    unsafe{outb(0x60, 0xED);} 
        let mut sector = [0u8; 512];
        device.read_sector(0, &mut sector); // Boot sector

        let bytes_per_sector = u16::from_le_bytes([sector[11], sector[12]]) as u32;
        let sectors_per_cluster = sector[13] as u32;
        let reserved_sectors = u16::from_le_bytes([sector[14], sector[15]]) as u32;
        let fat_count = sector[16] as u32;
        let root_dir_entries = u16::from_le_bytes([sector[17], sector[18]]) as u32;
        let fat_size = u16::from_le_bytes([sector[22], sector[23]]) as u32;

        let root_dir_sectors = ((root_dir_entries * 32) + (bytes_per_sector - 1)) / bytes_per_sector;
        let root_dir_start = reserved_sectors + fat_count * fat_size;
        let data_start = root_dir_start + root_dir_sectors;

        Self {
            device,
            fat_start: reserved_sectors,
            root_dir_start,
            data_start,
        }
    }

pub fn read_file(&mut self, name: &[u8; 11], buf: &mut [u8]) -> Option<usize> {
    let mut sector = [0u8; 512];
    
    for i in 0..14 {
        self.device.read_sector(self.root_dir_start + i, &mut sector);
        
        for j in 0..16 {
            let offset = j * 32;
            let entry = &sector[offset..offset + 32];
            let name_in_entry = &entry[0..11];
            if &sector[offset..offset + 11] == name ||  // Сравниваем с именем + расширением
               (&sector[offset..offset + 8] == &name[..8]) {
                let first_cluster = u16::from_le_bytes([sector[offset + 26], sector[offset + 27]]);
                let size = u32::from_le_bytes([
                    sector[offset + 28], sector[offset + 29],
                    sector[offset + 30], sector[offset + 31],
                ]) as usize;

                let mut fat_sector = [0u8; 512];
                let mut current_cluster = first_cluster;
                let mut bytes_read = 0;

                while bytes_read < size && current_cluster < 0xFF8 {
                    let lba = self.data_start.checked_add((current_cluster as u32).checked_sub(2)?)?;
                    self.device.read_sector(lba, &mut sector);
                    // copy
                    let mut bytes_to_copy = 512;
                    if 512 >  size - bytes_read {
                    	bytes_to_copy = size - bytes_read;
                    }
                    buf[bytes_read..bytes_read + bytes_to_copy]
                        .copy_from_slice(&sector[..bytes_to_copy]);
                    bytes_read += bytes_to_copy;

                    // next cluster
                    let fat_offset = current_cluster as usize * 3 / 2;
                    let fat_sector_num = fat_offset / 512;
                    let fat_entry_offset = fat_offset % 512;
                    self.device.read_sector(self.fat_start + fat_sector_num as u32, &mut fat_sector);
                    let fat_entry = u16::from_le_bytes([
                        fat_sector[fat_entry_offset],
                        fat_sector[fat_entry_offset + 1]
                    ]);
                    
                    current_cluster = if current_cluster & 1 != 0 {
                        (fat_entry >> 4) as u16
                    } else {
                        (fat_entry & 0x0FFF) as u16
                    };
                    // Check for end of chain or a bad cluster
                    if current_cluster >= 0xFF7 {
                        break;
                    }
                }
                return Some(size);
            }
        }
    }
    None
}

    pub fn write_file(&mut self, name: &[u8; 11], data: &[u8]) -> bool {
    	let mut sector = [0u8; 512];
        	for i in 0..14 {
            	self.device.read_sector(self.root_dir_start + i, &mut sector);
            	for j in 0..16 {
                	let offset = j * 32;
                	let entry = &sector[offset..offset + 11];
                	if entry == name {
                   		println!("File with the same name already exists!");
                    	return false;
                	}
            	}
        	}
        
        let mut fat_sector = [0u8; 512];
        for fat_index in 0..9 {
            self.device.read_sector(self.fat_start + fat_index, &mut fat_sector);
            for i in 0..(512 / 3) {
                let entry_index = i * 3;
                if entry_index + 2 >= 512 { break; }
    
                let a = fat_sector[entry_index];
                let b = fat_sector[entry_index + 1];
                let c = fat_sector[entry_index + 2];
    
                let cluster1 = ((b as u16 & 0x0F) << 8) | a as u16;
                let cluster2 = ((c as u16) << 4) | ((b as u16 & 0xF0) >> 4);
    
                if cluster1 == 0 {
                    let cluster_num = (i * 2) as u16;
    
                    fat_sector[entry_index] = 0xFF;
                    fat_sector[entry_index + 1] = (fat_sector[entry_index + 1] & 0xF0) | 0x0F;
    
                    self.device.write_sector(self.fat_start + fat_index, &fat_sector);
    
                    let lba = self.data_start + (cluster_num as u32 - 2);
                    let mut data_sector = [0u8; 512];
                    data_sector[..data.len()].copy_from_slice(data);
                    self.device.write_sector(lba, &data_sector);
    
                    let mut root_sector = [0u8; 512];
                    for i in 0..14 {
                        self.device.read_sector(self.root_dir_start + i, &mut root_sector);
                        for j in 0..16 {
                            let offset = j * 32;
                            if root_sector[offset] == 0x00 || root_sector[offset] == 0xE5 {
                                root_sector[offset..offset + 11].copy_from_slice(name);
                                root_sector[offset + 11] = 0x20; // artibut: standart file
    
                                // cluster
                                let cl = cluster_num.to_le_bytes();
                                root_sector[offset + 26] = cl[0];
                                root_sector[offset + 27] = cl[1];
    
                                // size
                                let size = (data.len() as u32).to_le_bytes();
                                root_sector[offset + 28..offset + 32].copy_from_slice(&size);
    
                                self.device.write_sector(self.root_dir_start + i, &root_sector);
                                return true;
                            }
                        }
                    }
    
                    return false; // root is full
                }
            }
        }
    
        false // no free clusters
    }

    pub fn list_files(&mut self) {
        let mut sector = [0u8; 512];
        for i in 0..14 {
            self.device.read_sector(self.root_dir_start + i, &mut sector);
            for j in 0..16 {
                let offset = j * 32;
                let entry = &sector[offset..offset + 32];
                let first_byte = entry[0];
    
                if first_byte == 0x00 {
                    return; // no more
                }
                if first_byte == 0xE5 {
                    continue; // deleted
                }
    
                let name = &entry[0..8];
                let ext = &entry[8..11];
                let size = u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]);
                let cluster = u16::from_le_bytes([entry[26], entry[27]]);
    
                let filename = core::str::from_utf8(name).unwrap_or("???").trim();
                let extension = core::str::from_utf8(ext).unwrap_or("").trim();
    
                println!("{}.{} (cluster {}, {} bytes)", filename, extension, cluster, size);
            }
        }
    }

    pub fn delete_file(&mut self, name: &[u8; 11]) -> bool {
        let mut sector = [0u8; 512];
        for i in 0..14 {
            self.device.read_sector(self.root_dir_start + i, &mut sector);
            for j in 0..16 {
                let offset = j * 32;
                if &sector[offset..offset + 11] == name {
                    sector[offset] = 0xE5; // Set as deleted
                    self.device.write_sector(self.root_dir_start + i, &sector);
    
                    return true;
                }
            }
        }
        println!("File not found");
        false
    }
}
