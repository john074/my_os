use crate::pci;
use crate::println;

pub const ETH_TYPE_IPV4: u16 = 0x0800;
pub const ETH_TYPE_ARP: u16 = 0x0806;
pub const ETH_HDR_LEN: usize = 14;
pub const IPV4_HDR_LEN: usize = 20;
pub const ARP_LEN: usize = 28;
pub const IP_PROTO_ICMP: u8 = 1;
pub const ICMP_ECHO_REQUEST: u8 = 8;
pub const ICMP_ECHO_REPLY: u8 = 0;
pub const ETH_BROADCAST: [u8;6] = [0xff;6];

const REG_CTRL: u32 = 0x0000;
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

pub static mut NIC_PTR: *mut E1000 = core::ptr::null_mut();

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
#[derive(Clone, Copy)]
pub struct IcmpEcho {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: u16,
    pub ident: u16,
    pub seq: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ArpPacket {
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

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Ipv4Header {
    vihl: u8,
    tos: u8,
    len: u16,
    id: u16,
    frag: u16,
    ttl: u8,
    proto: u8,
    checksum: u16,
    src: [u8;4],
    dst: [u8;4],
}

#[repr(C, packed)]
pub struct EthernetHeader {
    pub dst: [u8;6],
    pub src: [u8;6],
    pub ethertype: u16,
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
    for chunk in data.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            (chunk[0] as u16) << 8
        };

        sum += word as u32;
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

pub fn send_arp(nic: &mut E1000, op: u16, target_mac: [u8;6], target_ip: [u8;4]) {
    let mut buf = [0u8; 64];

    write_eth_header(&mut buf, target_mac, nic.mac, ETH_TYPE_ARP);

    let arp = ArpPacket {
        htype: 1u16.to_be(),
        ptype: ETH_TYPE_IPV4.to_be(),
        hlen: 6,
        plen: 4,
        oper: op.to_be(),
        sha: nic.mac,
        spa: nic.ip,
        tha: target_mac,
        tpa: target_ip,
    };

    let bytes = unsafe {
        core::slice::from_raw_parts(&arp as *const _ as *const u8, ARP_LEN)
    };

    buf[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN].copy_from_slice(bytes);
    nic.send(&buf[..(ETH_HDR_LEN + ARP_LEN)]);
}

pub fn ping(nic: &mut E1000, target_ip: [u8;4]) {
    if let Some(target_mac) = resolve(nic, target_ip) {
	    let seq = 1;
	  
	    send_ping(nic, target_ip, target_mac, seq);
    }
 	else {
 		println!("target not found");
 	}
}

pub fn send_ping(nic: &mut E1000, dst_ip: [u8;4], dst_mac: [u8;6], seq: u16) {
    let mut icmp = [0u8; 64];
    let icmp_len = build_icmp_request(seq, &mut icmp);

    let mut frame = [0u8; 128];

    let len = build_ipv4_packet(
        nic,
        &mut frame,
        dst_mac,
        dst_ip,
        IP_PROTO_ICMP,
        &icmp[..icmp_len],
    );

	println!("PING to {:?}", dst_ip);
    nic.send(&frame[..len]);
}

pub fn build_ipv4_packet(nic: &E1000, buf: &mut [u8], dst_mac: [u8;6], dst_ip: [u8;4], proto: u8, payload: &[u8]) -> usize {
    write_eth_header(buf, dst_mac, nic.mac, ETH_TYPE_IPV4);

    let ip = Ipv4Header {
        vihl: 0x45,
        tos: 0,
        len: (IPV4_HDR_LEN + payload.len()) as u16,
        id: 0,
        frag: 0,
        ttl: 64,
        proto,
        checksum: 0,
        src: nic.ip,
        dst: dst_ip,
    };

    let ip_bytes = unsafe {
        core::slice::from_raw_parts(&ip as *const _ as *const u8, IPV4_HDR_LEN)
    };

    let ip_start = ETH_HDR_LEN;
    buf[ip_start..ip_start + IPV4_HDR_LEN].copy_from_slice(ip_bytes);
    
    let csum = checksum(&buf[ip_start..ip_start + IPV4_HDR_LEN]);
    buf[ip_start + 10..ip_start + 12].copy_from_slice(&csum.to_be_bytes());
    
    let payload_start = ETH_HDR_LEN + IPV4_HDR_LEN;
    buf[payload_start..payload_start + payload.len()].copy_from_slice(payload);
    
    payload_start + payload.len()
}

pub fn resolve(nic: &mut E1000, target_ip: [u8;4]) -> Option<[u8;6]> {
    send_arp(nic, 1, ETH_BROADCAST, target_ip);

    for _ in 0..10_000_000 {
        if let Some(pkt) = nic.recv() {
            if pkt.len() < ETH_HDR_LEN + ARP_LEN {
                continue;
            }

            if let Some(ethertype) = ethernet_type(pkt) {
	            if ethertype != ETH_TYPE_ARP {
	                continue;
	            }
	        } else {
	        	continue;
	        }

            let arp = unsafe {
                &*(pkt[ETH_HDR_LEN..].as_ptr() as *const ArpPacket)
            };

            if u16::from_be(arp.oper) != 2 {
                continue;
            }

            if arp.spa == target_ip {
                return Some(arp.sha);
            }
        }
    }

    None
}

pub fn handle_packet(nic: &mut E1000, pkt: &[u8]) {
    if pkt.len() < 14 {
        return;
    }

    if let Some(ethertype) = ethernet_type(pkt) {
    	match ethertype {
    	    ETH_TYPE_ARP => handle_arp(nic, pkt),
    	    ETH_TYPE_IPV4 => handle_ipv4(nic, pkt),
    	    _ => {}
    	}	
    }
}

fn handle_arp(nic: &mut E1000, pkt: &[u8]) {
    if pkt.len() < 42 {
        return;
    }
    
    let arp = unsafe { &*(pkt[14..].as_ptr() as *const ArpPacket) };
    if u16::from_be(arp.oper) != 1 {
        return;
    }

    if arp.tpa != nic.ip {
        return;
    }

    send_arp_reply(nic, arp);
}

pub fn send_arp_reply(nic: &mut E1000, req: &ArpPacket) {
    send_arp(nic, 2, req.sha, req.spa);
}

fn handle_ipv4(nic: &mut E1000, pkt: &[u8]) {
    if pkt.len() < ETH_HDR_LEN + IPV4_HDR_LEN {
        return;
    }

    let ip = unsafe { &*(pkt[ETH_HDR_LEN..].as_ptr() as *const Ipv4Header) };
    if ip.dst != nic.ip {
        return;
    }

    if ip.proto == IP_PROTO_ICMP {
        handle_icmp(nic, pkt, ip);
    }
}

fn handle_icmp(nic: &mut E1000, pkt: &[u8], ip: &Ipv4Header) {
    let icmp_start = ETH_HDR_LEN + ((ip.vihl & 0x0F) as usize * 4);
    let icmp = &pkt[icmp_start..];

    match icmp[0] {
        ICMP_ECHO_REQUEST => send_icmp_reply(nic, pkt, ip),
        ICMP_ECHO_REPLY => println!("PING reply from {:?}", ip.src),
        _ => {}
    }
}

pub fn send_icmp_reply(nic: &mut E1000, pkt: &[u8], ip: &Ipv4Header) {
    let mut buf = [0u8;1500];

    let len = pkt.len();
    buf[..len].copy_from_slice(pkt);

    write_eth_header(
        &mut buf,
        pkt[6..12].try_into().unwrap(),
        nic.mac,
        ETH_TYPE_IPV4
    );

    let ip_start = ETH_HDR_LEN;
    buf[ip_start + 12..ip_start + 16].copy_from_slice(&nic.ip);
    buf[ip_start + 16..ip_start + 20].copy_from_slice(&ip.src);
    
    let ihl = (ip.vihl & 0x0F) as usize * 4;
    let icmp_start = ETH_HDR_LEN + ihl;
    let icmp = &mut buf[icmp_start..len];

    icmp[0] = ICMP_ECHO_REPLY;
    icmp[2] = 0;
    icmp[3] = 0;

    let csum = checksum(icmp);
    icmp[2..4].copy_from_slice(&csum.to_be_bytes());
    nic.send(&buf[..len]);
}

pub fn write_eth_header(buf: &mut [u8], dst: [u8;6], src: [u8;6], ethertype: u16) {
    let hdr = EthernetHeader { dst, src, ethertype: ethertype.to_be() };

    let bytes = unsafe {
        core::slice::from_raw_parts(&hdr as *const _ as *const u8, core::mem::size_of::<EthernetHeader>())
    };

    buf[..ETH_HDR_LEN].copy_from_slice(bytes);
}

pub fn ethernet_type(pkt: &[u8]) -> Option<u16> {
    if pkt.len() < ETH_HDR_LEN {
        return None;
    }

    Some(u16::from_be_bytes([pkt[12], pkt[13]]))
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
