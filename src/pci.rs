const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
}

impl PciDevice {
    pub fn read(&self, offset: u8) -> u32 {
        let address: u32 = (1 << 31)
            | ((self.bus as u32) << 16)
            | ((self.slot as u32) << 11)
            | ((self.func as u32) << 8)
            | ((offset as u32) & 0xfc);

        unsafe {
            x86::io::outl(PCI_CONFIG_ADDRESS, address);
            x86::io::inl(PCI_CONFIG_DATA)
        }
    }

    pub fn write(&self, offset: u8, value: u32) {
        let address: u32 = (1 << 31)
            | ((self.bus as u32) << 16)
            | ((self.slot as u32) << 11)
            | ((self.func as u32) << 8)
            | ((offset as u32) & 0xfc);
    
        unsafe {
            x86::io::outl(PCI_CONFIG_ADDRESS, address);
            x86::io::outl(PCI_CONFIG_DATA, value);
        }
    }

    pub fn bar0(&self) -> u32 {
        self.read(0x10) & 0xFFFFFFF0
    }

    pub fn enable(&self) {
        let mut cmd = self.read(0x04);
        cmd |= 1 << 1; // memory space
        cmd |= 1 << 2; // bus master
        self.write(0x04, cmd);
    }
}

pub fn find_device(vendor: u16, device: u16) -> Option<PciDevice> {
    for bus in 0..=255 {
        for slot in 0..32 {
            let dev = PciDevice { bus, slot, func: 0 };
            let id = dev.read(0);
            let ven = (id & 0xffff) as u16;
            let dev_id = (id >> 16) as u16;

            if ven == vendor && dev_id == device {
                return Some(dev);
            }
        }
    }
    None
}
