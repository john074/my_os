use crate::pci;
use crate::println;

pub const ETH_TYPE_IPV4: u16 = 0x0800;
pub const ETH_TYPE_ARP: u16 = 0x0806;
pub const IP_PROTO_ICMP: u8 = 1;
pub const ICMP_ECHO_REQUEST: u8 = 8;
pub const ICMP_ECHO_REPLY: u8 = 0;

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
const REG_TIPG: u32 = 0x0410;

const RX_RING: usize = 32;
const TX_RING: usize = 8;

const RX_BUFFER_SIZE: usize = 2048;

static mut RX_DESC: [RxDesc; RX_RING] = [RxDesc { addr:0, length:0, checksum:0, status:0, errors:0, special:0 }; RX_RING];
static mut TX_DESC: [TxDesc; TX_RING] = [TxDesc { addr:0, length:0, cso:0, cmd:0, status:1, css:0, special:0 }; TX_RING];
static mut RX_BUF: [[u8; RX_BUFFER_SIZE]; RX_RING] = [[0; RX_BUFFER_SIZE]; RX_RING];
static mut TX_BUF: [[u8; RX_BUFFER_SIZE]; TX_RING] = [[0; RX_BUFFER_SIZE]; TX_RING];

#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct RxDesc {
    pub addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[repr(C, align(16))]
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

#[repr(C, packed)]
pub struct IcmpEcho {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: u16,
    pub ident: u16,
    pub seq: u16,
}

#[repr(C, packed)]
struct ArpPacket {
    htype: u16,
    ptype: u16,
    hlen: u8,
    plen: u8,
    oper: u16,
    sha: [u8;6],
    spa: [u8;4],
    tha: [u8;6],
    tpa: [u8;4],
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
    ip: [u8;4],
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
	    self.write(REG_TIPG, 0x0060200A);
	
	    let tctl =
	        (1 << 1) |  // EN
	        (1 << 3) |  // PSP
	        (15 << 4) | // CT
	        (64 << 12); // COLD
	
	    self.write(REG_TCTL, tctl);
	}
	
	//#[allow(static_mut_refs)]
	pub fn send(&mut self, data: &[u8]) {
	    let i = self.tx_tail;
	    self.tx_buf[i][..data.len()].copy_from_slice(data);
	    self.tx_desc[i].addr = self.tx_buf[i].as_ptr() as u64;
	    
	    let len = core::cmp::max(60, data.len());
	    self.tx_buf[i][..data.len()].copy_from_slice(data);
	    for b in &mut self.tx_buf[i][data.len()..len] {
	        *b = 0;
	    }
	    
	    self.tx_desc[i].length = len as u16;
	    self.tx_desc[i].cmd =
	        (1 << 0) | // EOP
	        (1 << 1) | // IFCS
	        (1 << 3);  // RS
	
	    self.tx_desc[i].status = 0;
	    self.tx_tail = (self.tx_tail + 1) % TX_RING;
	    self.write(REG_TDT, self.tx_tail as u32);
	    
	    // println!("TDH {:x}", self.read(REG_TDH));
	    // println!("TDT {:x}", self.read(REG_TDT));
	    // println!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
	    // 	self.mac[0],self.mac[1],self.mac[2],self.mac[3],self.mac[4],self.mac[5]);
	    // println!("TCTL {:x}", self.read(REG_TCTL));
	    // println!("STATUS {:x}", self.read(0x0008));
	    // println!("-111");
	    // let framebuffer = unsafe { framebuffer::FRAMEBUFFER.as_mut().unwrap() };
	    // framebuffer.draw_frame();
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
	pub fn init(regs: *mut u32, ip: [u8;4]) -> Self {
	    let mut dev = Self {
	        regs,
	        rx_desc: unsafe { &mut RX_DESC },
	        tx_desc: unsafe { &mut TX_DESC },
	        rx_buf: unsafe { &mut RX_BUF },
	        tx_buf: unsafe { &mut TX_BUF },
	        rx_tail: 0,
	        tx_tail: 0,
	        mac: [0;6],
	        ip,
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

	pub fn init_from_pci(ip: [u8;4]) -> Self {
	    let dev = pci::find_device(0x8086, 0x100e).unwrap();
	    dev.enable();
	    let bar0 = dev.bar0();
	    let mmio = bar0 as *mut u32;
	    E1000::init(mmio, ip)
	}
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        let word = ((data[i] as u16) << 8) | data[i+1] as u16;
        sum += word as u32;
        i += 2;
    }

    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}

pub fn build_icmp_request(seq: u16, buf: &mut [u8]) -> usize {
    let hdr = IcmpEcho {
        icmp_type: ICMP_ECHO_REQUEST,
        code: 0,
        checksum: 0,
        ident: 0x1234,
        seq,
    };

    let hdr_bytes = unsafe { core::slice::from_raw_parts(&hdr as *const _ as *const u8, core::mem::size_of::<IcmpEcho>()) };

    buf[..hdr_bytes.len()].copy_from_slice(hdr_bytes);
    let data = b"hello";

    buf[hdr_bytes.len()..hdr_bytes.len()+data.len()].copy_from_slice(data);
    let len = hdr_bytes.len() + data.len();
    let sum = checksum(&buf[..len]);
    buf[2..4].copy_from_slice(&sum.to_be_bytes());

    len
}

pub fn ping(nic: &mut E1000, target_ip: [u8;4]) {
    if let Some(target_mac) = resolve(nic, target_ip) {
	    let seq = 1;
	  
	    send_ping(nic, target_ip, target_mac, seq);
	    if wait_ping_reply(nic, seq) {
	        println!("ping reply received");
	    } 
	    else {
	        println!("timeout");
	    }
    }
 	else {
 		println!("target not found");
 	}
}

pub fn send_ping(nic: &mut E1000, dst_ip: [u8;4], dst_mac: [u8;6], seq: u16) {
    let mut icmp = [0u8;64];
    let icmp_len = build_icmp_request(seq, &mut icmp);
    let mut frame = [0u8;128];
    let eth_len = build_ipv4_frame(
    		nic,
            frame.as_mut_slice(),
            nic.read_mac(),
            dst_mac,
            dst_ip,
            IP_PROTO_ICMP,
            &icmp[..icmp_len],
        );
        
    nic.send(&frame[..eth_len]);
}

pub fn wait_ping_reply(nic: &mut E1000, seq: u16) -> bool {
    for _ in  0..100000 {
        if let Some(pkt) = nic.recv() {
            if let Some(reply_seq) = parse_echo_reply(pkt) {
                if reply_seq == seq {
                    return true;
                }
            }
        }
    }
    false
}

pub fn build_ipv4_frame(nic: &mut E1000, buf: &mut [u8], src_mac: [u8;6], dst_mac: [u8;6], dst_ip: [u8;4], proto: u8, payload: &[u8]) -> usize {
    let src_ip = nic.ip;

    // ethernet
    buf[0..6].copy_from_slice(&dst_mac);
    buf[6..12].copy_from_slice(&src_mac);
    buf[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    
    let ip = &mut buf[14..];
    ip[0] = 0x45;
    ip[1] = 0;
    
    let total_len = (20 + payload.len()) as u16;
    ip[2..4].copy_from_slice(&total_len.to_be_bytes());
    ip[4..6].copy_from_slice(&0u16.to_be_bytes());
    ip[6..8].copy_from_slice(&0u16.to_be_bytes());
    
    ip[8] = 64;
    ip[9] = proto;
    
    ip[10..12].copy_from_slice(&0u16.to_be_bytes());
    ip[12..16].copy_from_slice(&src_ip);
    ip[16..20].copy_from_slice(&dst_ip);

    let csum = checksum(&ip[..20]);
    ip[10..12].copy_from_slice(&csum.to_be_bytes());
    ip[20..20+payload.len()].copy_from_slice(payload);

    14 + 20 + payload.len()
}

pub fn parse_echo_reply(pkt: &[u8]) -> Option<u16> {
    if pkt.len() < 42 {
        return None;
    }

    let ethertype = u16::from_be_bytes([pkt[12], pkt[13]]);
    if ethertype != ETH_TYPE_IPV4 {
        return None;
    }

    let ip = &pkt[14..];
    if ip[9] != 1 {
        return None;
    }

    let icmp = &ip[20..];
    if icmp[0] != 0 {
        return None;
    }

    let seq = u16::from_be_bytes([icmp[6], icmp[7]]);
    Some(seq)
}

pub fn resolve(nic: &mut E1000, target_ip: [u8;4]) -> Option<[u8;6]> {
    let mut buf = [0u8; 64];
    let my_ip = nic.ip;
    let arp = ArpPacket {
        htype: 1u16.to_be(),
        ptype: 0x0800u16.to_be(),
        hlen: 6,
        plen: 4,
        oper: 1u16.to_be(),
        sha: nic.mac,
        spa: my_ip,
        tha: [0;6],
        tpa: target_ip,
    };

    let arp_bytes = unsafe { core::slice::from_raw_parts(&arp as *const _ as *const u8, core::mem::size_of::<ArpPacket>()) };

    buf[0..6].copy_from_slice(&[0xff;6]);
    buf[6..12].copy_from_slice(&nic.read_mac());
    buf[12..14].copy_from_slice(&0x0806u16.to_be_bytes());
    buf[14..14+arp_bytes.len()].copy_from_slice(arp_bytes);

    nic.send(&buf[..42]);

    for _ in 0..100000 {
        if let Some(pkt) = nic.recv() {
            let ethertype = u16::from_be_bytes([pkt[12], pkt[13]]);
            if ethertype != ETH_TYPE_ARP {
                continue;
            }
			
            let arp = unsafe { &*(pkt[14..].as_ptr() as *const ArpPacket) };
            if arp.oper.to_be() != 2 {     	
                continue;
            }
			
            if arp.spa == target_ip {
                return Some(arp.sha);
            }
        }
    }

    None
}

pub fn parse_ip(ip_str: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut octet_index = 0;
    let mut current_octet = 0u16;
    
    for (i, c) in ip_str.trim().chars().enumerate() {
        match c {
            '0'..='9' => {
                current_octet = current_octet * 10 + (c as u16 - '0' as u16);
                if current_octet > 255 {
                    return None;
                }
            }
            '.' => {
                if i == 0 || octet_index >= 3 {
                    return None;
                }
                octets[octet_index] = current_octet as u8;
                octet_index += 1;
                current_octet = 0;
            }
            _ => return None,
        }
    }
    
    if octet_index != 3 {
        return None;
    }
    octets[3] = current_octet as u8;
    
    Some(octets)
}
