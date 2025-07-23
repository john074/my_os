//
//
//	WARNING: THIS IMPLEMENTATION DOESNT WORK PROPERLY. 
//	SYSCALLS FROM PROGRAMS READ WITH THIS WILL CAUSE PAGE FAULT.
//	USE FAT12 OR FAT32 INSTEAD.
//
//
use x86::io::{ inb, inw, outb, outw };
use xmas_elf::ElfFile;
use xmas_elf::program::Type;
use core::ptr::copy_nonoverlapping;

use crate::println;
use crate::print;
use crate::multitasking;
use crate::memory;

pub static mut RETURN_ADDR: usize = 0;
pub static mut EXECUTOR_PTR: *mut multitasking::Executor = core::ptr::null_mut();

// pub fn load_elf_and_jump(elf_data: &[u8], executor: *mut multitasking::Executor) {
// 	unsafe{
// 		EXECUTOR_PTR = executor;
// 	};
// 	
//     let elf = ElfFile::new(elf_data).expect("Invalid ELF");
// 
//     for ph in elf.program_iter() {
//         if ph.get_type().unwrap() != Type::Load {
//             continue;
//         }
// 
//         let p_offset = ph.offset() as usize;
//         let p_vaddr = ph.virtual_addr() as usize;
//         let p_filesz = ph.file_size() as usize;
//         let p_memsz = ph.mem_size() as usize;
//         println!("PVADR: {}", p_vaddr);
// 
//         // print!("SEGMENT: ");
//         // print!("{} ", p_offset as u64);
//        	// print!("{} ", p_vaddr as u64);
//        	// print!("{} ", p_filesz as u64);
//        	// print!("{} ", p_memsz as u64);
// 
// //         unsafe {
// //             let src = elf_data.as_ptr().add(p_offset);
// //             let dst = p_vaddr as *mut u8;
// //             copy_nonoverlapping(src, dst, p_filesz);
// // 
// //             core::ptr::write_bytes(dst.add(p_filesz), 0, p_memsz - p_filesz);
// //         }
// 
// 		unsafe {
//         	let dst = p_vaddr as *mut u8;
// 
//         	if p_filesz > 0 {
//             	let src = elf_data.as_ptr().add(p_offset);
//             	copy_nonoverlapping(src, dst, p_filesz);
//         	}
// 
//         	// обязательно обнуляем оставшуюся часть
//         	if p_memsz > p_filesz {
//             	core::ptr::write_bytes(dst.add(p_filesz), 0, p_memsz - p_filesz);
//         	}
//     	}
//     }
// 
// 	let entry = elf.header.pt2.entry_point() as usize;
// 	unsafe {
// 	    RETURN_ADDR = return_to_kernel as usize;
// 	    core::arch::asm!(
// 	        "mov rax, {0}", // entry point
// 	        "jmp rax",
// 	        in(reg) entry,
// 	        options(noreturn)
// 	    );
// 	}
// }

use alloc::alloc::{ GlobalAlloc, Layout };

pub fn load_elf_and_jump(elf_data: &[u8], executor: *mut multitasking::Executor) {
    unsafe {
        EXECUTOR_PTR = executor;
    }

    let elf = ElfFile::new(elf_data).expect("Invalid ELF");

    let mut total_size = 0usize;
    let mut max_align = 0usize;

    for ph in elf.program_iter() {
        if ph.get_type().unwrap() == Type::Load {
            println!("Segment at {:x}, memsz {:x}, filesz {:x}, flags: {:?}", ph.virtual_addr(), ph.mem_size(), ph.file_size(), ph.flags());
        }
    }

    for ph in elf.program_iter() {
        if ph.get_type().unwrap() != Type::Load {
            continue;
        }

        let end = (ph.virtual_addr() + ph.mem_size()) as usize;
        total_size = total_size.max(end);
        max_align = max_align.max(ph.align() as usize);
    }

    let layout = Layout::from_size_align(total_size, max_align.max(0x1000)).unwrap();
    let base_ptr = unsafe { memory::ALLOCATOR.alloc(layout) };

    if base_ptr.is_null() {
        panic!("Failed to allocate memory for ELF program");
    }

    for ph in elf.program_iter() {
        if ph.get_type().unwrap() != Type::Load {
            continue;
        }

        let p_offset = ph.offset() as usize;
        let p_vaddr = ph.virtual_addr() as usize;
        let p_filesz = ph.file_size() as usize;
        let p_memsz = ph.mem_size() as usize;

        unsafe {
            let dst = base_ptr.add(p_vaddr);
            let src = elf_data.as_ptr().add(p_offset);

            copy_nonoverlapping(src, dst, p_filesz);

            if p_memsz > p_filesz {
                core::ptr::write_bytes(dst.add(p_filesz), 0, p_memsz - p_filesz);
            }
        }
    }

    let entry = unsafe { base_ptr.add(elf.header.pt2.entry_point() as usize) };

    unsafe {
        RETURN_ADDR = return_to_kernel as usize;
        core::arch::asm!(
            "mov rax, {0}",
            "jmp rax",
            in(reg) entry,
            options(noreturn)
        );
    }
}



extern "C" fn return_to_kernel() {
	let executor = unsafe { &mut *EXECUTOR_PTR };
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

pub struct Fat16Fs<D: BlockDevice> {
        device: D,
        fat_start: u32,
        root_dir_start: u32,
        data_start: u32,
        root_dir_sectors: u32,
    }
    
    impl<D: BlockDevice> Fat16Fs<D> {
        pub fn new(mut device: D) -> Self {
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
    
            Fat16Fs {
                device,
                fat_start: reserved_sectors,
                root_dir_start,
                data_start,
                root_dir_sectors,
            }
        }
    
        pub fn read_file(&mut self, name: &[u8; 11], buf: &mut [u8]) -> Option<usize> {
            let mut sector = [0u8; 512];
            for i in 0..self.root_dir_sectors {
                self.device.read_sector(self.root_dir_start + i, &mut sector);
                for j in 0..16 {
                    let offset = j * 32;
                    if &sector[offset..offset+11] == name {
                        let first_cluster = u16::from_le_bytes([sector[offset+26], sector[offset+27]]);
                        let size = u32::from_le_bytes([
                            sector[offset+28], sector[offset+29],
                            sector[offset+30], sector[offset+31],
                        ]) as usize;
                        let mut current_cluster = first_cluster;
                        let mut bytes_read = 0;
                        while bytes_read < size && current_cluster < 0xFFF8 {
                            let lba = self.data_start + (current_cluster as u32 - 2);
                            self.device.read_sector(lba, &mut sector);
                            let bytes_to_copy = core::cmp::min(512, size - bytes_read);
                            buf[bytes_read..bytes_read + bytes_to_copy]
                                .copy_from_slice(&sector[..bytes_to_copy]);
                            bytes_read += bytes_to_copy;
                            let fat_offset = (current_cluster as usize) * 2;
                            let fat_sector = (fat_offset / 512) as u32;
                            let ent_offset = fat_offset % 512;
                            self.device.read_sector(self.fat_start + fat_sector, &mut sector);
                            let next_cluster = u16::from_le_bytes([
                                sector[ent_offset],
                                sector[ent_offset + 1]
                            ]);
                            current_cluster = next_cluster;
                        }
                        return Some(size);
                    }
                }
            }
            None
        }

        // pub fn read_file(&mut self, name: &[u8; 11], buf: &mut [u8]) -> Option<usize> {
        //     let mut dir_sector = [0u8; 512];
        //     let mut fat_sector = [0u8; 512];
        //     let mut data_sector = [0u8; 512];
        // 
        //     // Ищем файл в корневом каталоге
        //     for i in 0..self.root_dir_sectors {
        //         self.device.read_sector(self.root_dir_start + i, &mut dir_sector);
        // 
        //         for j in 0..16 {
        //             let offset = j * 32;
        //             if &dir_sector[offset..offset + 11] == name {
        //                 let first_cluster = u16::from_le_bytes([
        //                     dir_sector[offset + 26],
        //                     dir_sector[offset + 27],
        //                 ]);
        //                 let size = u32::from_le_bytes([
        //                     dir_sector[offset + 28],
        //                     dir_sector[offset + 29],
        //                     dir_sector[offset + 30],
        //                     dir_sector[offset + 31],
        //                 ]) as usize;
        // 
        //                 let mut current_cluster = first_cluster;
        //                 let mut bytes_read = 0;
        // 
        //                 while bytes_read < size && current_cluster < 0xFFF8 {
        //                     // Читаем данные из текущего кластера
        //                     let lba = self.data_start + (current_cluster as u32 - 2);
        //                     self.device.read_sector(lba, &mut data_sector);
        // 
        //                     let bytes_to_copy = core::cmp::min(512, size - bytes_read);
        //                     buf[bytes_read..bytes_read + bytes_to_copy]
        //                         .copy_from_slice(&data_sector[..bytes_to_copy]);
        // 
        //                     bytes_read += bytes_to_copy;
        // 
        //                     // Читаем следующую запись FAT
        //                     let fat_offset = (current_cluster as usize) * 2;
        //                     let fat_sector_num = (fat_offset / 512) as u32;
        //                     let ent_offset = fat_offset % 512;
        // 
        //                     self.device.read_sector(self.fat_start + fat_sector_num, &mut fat_sector);
        // 
        //                     let next_cluster = u16::from_le_bytes([
        //                         fat_sector[ent_offset],
        //                         fat_sector[ent_offset + 1],
        //                     ]);
        // 
        //                     if next_cluster >= 0xFFF8 {
        //                         break;
        //                     }
        // 
        //                     current_cluster = next_cluster;
        //                 }
        // 
        //                 return Some(size);
        //             }
        //         }
        //     }
        // 
        //     None
        // }
    
        pub fn write_file(&mut self, name: &[u8; 11], data: &[u8]) -> bool {
            let mut sector = [0u8; 512];
            for i in 0..self.root_dir_sectors {
                self.device.read_sector(self.root_dir_start + i, &mut sector);
                for j in 0..16 {
                    if &sector[j*32..j*32+11] == name {
                        return false;
                    }
                }
            }
            let mut fat_sector_buf = [0u8; 512];
            for fat_index in 0.. { 
                if fat_index >=  self.root_dir_sectors * 0 { break; }
                self.device.read_sector(self.fat_start + fat_index, &mut fat_sector_buf);
                for i in 0..256 {
                    let entry_offset = i * 2;
                    let cluster_val = u16::from_le_bytes([
                        fat_sector_buf[entry_offset],
                        fat_sector_buf[entry_offset + 1]
                    ]);
                    let cluster_num = (fat_index * 256) as u32 + i as u32;
                    if cluster_num < 2 { continue; }
                    if cluster_val == 0 {
                        fat_sector_buf[entry_offset] = 0xFF;
                        fat_sector_buf[entry_offset + 1] = 0xFF;
                        self.device.write_sector(self.fat_start + fat_index, &fat_sector_buf);
                        let data_lba = self.data_start + (cluster_num - 2);
                        let mut data_buf = [0u8; 512];
                        let to_copy = core::cmp::min(data.len(), 512);
                        data_buf[..to_copy].copy_from_slice(&data[..to_copy]);
                        self.device.write_sector(data_lba, &data_buf);
                        for k in 0..self.root_dir_sectors {
                            self.device.read_sector(self.root_dir_start + k, &mut sector);
                            for l in 0..16 {
                                let off = l*32;
                                if sector[off] == 0x00 || sector[off] == 0xE5 {
                                    sector[off..off+11].copy_from_slice(name);
                                    sector[off+11] = 0x20; // атрибут (обычный файл)
                                    let cl_bytes = (cluster_num as u16).to_le_bytes();
                                    sector[off+26] = cl_bytes[0];
                                    sector[off+27] = cl_bytes[1];
                                    let size_bytes = (data.len() as u32).to_le_bytes();
                                    sector[off+28..off+32].copy_from_slice(&size_bytes);
                                    self.device.write_sector(self.root_dir_start + k, &sector);
                                    return true;
                                }
                            }
                        }
                        return false;
                    }
                }
            }
            false
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
