use crate::pci;

const REG_CTRL: u32 = 0x0000;
const REG_STATUS: u32 = 0x0008;
const REG_RCTL: u32 = 0x0100;
const REG_TCTL: u32 = 0x0400;
const REG_RDBAL: u32 = 0x2800;
const REG_RDBAH: u32 = 0x2804;
const REG_RDLEN: u32 = 0x2808;
const REG_RDH: u32 = 0x2810;
const REG_RDT: u32 = 0x2818;
const REG_TDBAL: u32 = 0x3800;
const REG_TDBAH: u32 = 0x3804;
const REG_TDLEN: u32 = 0x3808;
const REG_TDH: u32 = 0x3810;
const REG_TDT: u32 = 0x3818;
const REG_RAL: u32 = 0x5400;
const REG_RAH: u32 = 0x5404;

const RX_RING: usize = 32;
const TX_RING: usize = 8;

const RX_BUFFER_SIZE: usize = 2048;

static mut RX_DESC: [RxDesc; RX_RING] = [RxDesc { addr:0,length:0,checksum:0,status:0,errors:0,special:0 }; RX_RING];
static mut TX_DESC: [TxDesc; TX_RING] = [TxDesc { addr:0,length:0,cso:0,cmd:0,status:1,css:0,special:0 }; TX_RING];
static mut RX_BUF: [[u8; RX_BUFFER_SIZE]; RX_RING] = [[0; RX_BUFFER_SIZE]; RX_RING];
static mut TX_BUF: [[u8; RX_BUFFER_SIZE]; TX_RING] = [[0; RX_BUFFER_SIZE]; TX_RING];

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RxDesc {
    pub addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TxDesc {
    pub addr: u64,
    pub length: u16,
    pub cso: u8,
    pub cmd: u8,
    pub status: u8,
    pub css: u8,
    pub special: u16,
}

pub struct E1000 {
    regs: *mut u32,
    rx_desc: &'static mut [RxDesc; RX_RING],
    tx_desc: &'static mut [TxDesc; TX_RING],
    rx_buf: &'static mut [[u8; RX_BUFFER_SIZE]; RX_RING],
    tx_buf: &'static mut [[u8; RX_BUFFER_SIZE]; TX_RING],
    rx_tail: usize,
    tx_tail: usize,
    mac: [u8;6],
}

impl E1000 {
    fn read(&self, reg: u32) -> u32 {
        unsafe {
            core::ptr::read_volatile(self.regs.add((reg/4) as usize))
        }
    }

    fn write(&self, reg: u32, value: u32) {
        unsafe {
            core::ptr::write_volatile(self.regs.add((reg/4) as usize), value)
        }
    }

	fn read_mac(&self) -> [u8;6] {
	    let ral = self.read(REG_RAL);
	    let rah = self.read(REG_RAH);
	
	    [
	        (ral & 0xff) as u8,
	        ((ral >> 8) & 0xff) as u8,
	        ((ral >> 16) & 0xff) as u8,
	        ((ral >> 24) & 0xff) as u8,
	        (rah & 0xff) as u8,
	        ((rah >> 8) & 0xff) as u8,
	    ]
	}

	fn init_rx(&mut self) {
	    for i in 0..RX_RING {
	        self.rx_desc[i].addr = self.rx_buf[i].as_ptr() as u64;
	        self.rx_desc[i].status = 0;
	    }
	
	    let addr = self.rx_desc.as_ptr() as u64;
	    self.write(REG_RDBAL, addr as u32);
	    self.write(REG_RDBAH, (addr >> 32) as u32);
	    self.write(REG_RDLEN, (RX_RING * core::mem::size_of::<RxDesc>()) as u32);
	    self.write(REG_RDH, 0);
	    self.write(REG_RDT, (RX_RING - 1) as u32);
	
	    let rctl =
	        (1 << 1) | // enable
	        (1 << 15); // broadcast accept
	
	    self.write(REG_RCTL, rctl);
	}

	fn init_tx(&mut self) {
	    for i in 0..TX_RING {
	        self.tx_desc[i].status = 1;
	    }
	
	    let addr = self.tx_desc.as_ptr() as u64;
	    self.write(REG_TDBAL, addr as u32);
	    self.write(REG_TDBAH, (addr >> 32) as u32);
	    self.write(REG_TDLEN, (TX_RING * core::mem::size_of::<TxDesc>()) as u32);
	    self.write(REG_TDH, 0);
	    self.write(REG_TDT, 0);
	
	    let tctl =
	        (1 << 1) | // enable
	        (1 << 3);  // pad short packets
	
	    self.write(REG_TCTL, tctl);
	}

	pub fn send(&mut self, data: &[u8]) {
	    let i = self.tx_tail;
	    self.tx_buf[i][..data.len()].copy_from_slice(data);
	    self.tx_desc[i].addr = self.tx_buf[i].as_ptr() as u64;
	    self.tx_desc[i].length = data.len() as u16;
	    self.tx_desc[i].cmd =
	        (1 << 0) | // EOP
	        (1 << 3);  // RS
	
	    self.tx_desc[i].status = 0;
	    self.tx_tail = (self.tx_tail + 1) % TX_RING;
	    self.write(REG_TDT, self.tx_tail as u32);
	}

	pub fn recv(&mut self) -> Option<&[u8]> {
	    let i = self.rx_tail;
	    if self.rx_desc[i].status & 1 == 0 {
	        return None;
	    }
	
	    let len = self.rx_desc[i].length as usize;
	    let packet = &self.rx_buf[i][..len];
	    self.rx_desc[i].status = 0;
	    self.rx_tail = (self.rx_tail + 1) % RX_RING;
	    self.write(REG_RDT, i as u32);
	    Some(packet)
	}

	#[allow(static_mut_refs)]
	pub fn init(regs: *mut u32) -> Self {
	    let mut dev = Self {
	        regs,
	        rx_desc: unsafe { &mut RX_DESC },
	        tx_desc: unsafe { &mut TX_DESC },
	        rx_buf: unsafe { &mut RX_BUF },
	        tx_buf: unsafe { &mut TX_BUF },
	        rx_tail: 0,
	        tx_tail: 0,
	        mac: [0;6],
	    };

	    // reset
	    dev.write(REG_CTRL, 1 << 26);

	    for _ in 0..100000 {
	        core::hint::spin_loop();
	    }

	    dev.mac = dev.read_mac();
	    dev.init_rx();
	    dev.init_tx();

	    dev
	}

	pub fn init_from_pci() -> Self {
	    let dev = pci::find_device(0x8086, 0x100e).unwrap();
	    dev.enable();
	    let bar0 = dev.bar0();
	    let mmio = bar0 as *mut u32;
	    E1000::init(mmio)
	}
}
