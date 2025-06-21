use x86::io::{ inb, inw, outb };
use crate::println;

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
    	unimplemented!();        
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
        // Read root
        let mut sector = [0u8; 512];
        for i in 0..14 { // 224 entries / 16 per sector = 14 sectors
            self.device.read_sector(self.root_dir_start + i, &mut sector);
            for j in 0..16 {
                let offset = j * 32;
                if &sector[offset..offset + 11] == name {
                    let cluster = u16::from_le_bytes([sector[offset + 26], sector[offset + 27]]);
                    let size = u32::from_le_bytes([
                        sector[offset + 28], sector[offset + 29],
                        sector[offset + 30], sector[offset + 31],
                    ]) as usize;

                    let lba = self.data_start + ((cluster - 2) as u32);
                    let mut data_sector = [0u8; 512];
                    self.device.read_sector(lba, &mut data_sector);
                    buf[..size].copy_from_slice(&data_sector[..size]);
                    return Some(size);
                }
            }
        }
        None
    }

    pub fn write_file(&mut self, name: &[u8; 11], data: &[u8]) -> bool {
    	unimplemented!();
    }
}
