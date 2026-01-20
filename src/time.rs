use core::arch::asm;
use x86::io::{inb, outb};
use core::sync::atomic::{ AtomicU32 };

pub const TICKS_PER_SEC: u32 = 18;
pub const TICKS_PER_MIN: u32 = TICKS_PER_SEC * 60;
const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

pub static TICKS: AtomicU32 = AtomicU32::new(0);
pub static mut SECONDS: u8 = 0;
pub static mut MINUTES: u8 = 0;
pub static mut HOURS: u8 = 0;
pub static mut DAY: u8 = 0;
pub static mut MONTH: u8 = 0;
pub static mut YEAR: u16 = 0;

pub unsafe fn init() {
    let rtc = read_rtc();

    SECONDS = rtc.sec;
    MINUTES = rtc.min;
    HOURS   = rtc.hour;
    DAY     = rtc.day;
    MONTH   = rtc.month;
    YEAR    = rtc.year;
}

pub unsafe fn resync_from_cmos() {
	init();
}

unsafe fn cmos_read(reg: u8) -> u8 {
	outb(0x70, reg);
	inb(0x71)
}

unsafe fn cmos_update_in_progress() -> bool {
	cmos_read(0x0A) & 0x80 != 0
}

#[inline]
fn bcd_to_bin(x: u8) -> u8 {
	(x & 0x0F) + ((x >> 4) * 10)
}

#[derive(Clone, Copy, PartialEq)]
pub struct RtcTime {
	pub sec: u8,
	pub min: u8,
	pub hour: u8,
	pub day: u8,
	pub month: u8,
	pub year: u16,
}

pub unsafe fn read_rtc() -> RtcTime {
	let mut t1;
	let mut t2;

	loop {
		while cmos_update_in_progress() {}

		t1 = RtcTime {
			sec: cmos_read(0x00),
			min: cmos_read(0x02),
			hour: cmos_read(0x04),
			day: cmos_read(0x07),
			month: cmos_read(0x08),
			year: cmos_read(0x09) as u16
		};

		while cmos_update_in_progress() {}

		t2 = RtcTime {
			sec: cmos_read(0x00),
			min: cmos_read(0x02),
			hour: cmos_read(0x04),
			day: cmos_read(0x07),
			month: cmos_read(0x08),
			year: cmos_read(0x09) as u16
		};

		if t1 == t2 {
			break;
		}
	}

	let status_b = cmos_read(0x0B);
	let is_bcd = status_b & 0x04 == 0;
	let is_24h = status_b & 0x02 != 0;

	let mut t = t1;

	if is_bcd {
		t.sec = bcd_to_bin(t.sec);
		t.min = bcd_to_bin(t.min);
		t.hour = bcd_to_bin(t.hour & 0x7F) | (t.hour & 0x80);
		t.day = bcd_to_bin(t.day);
		t.month = bcd_to_bin(t.month);
		t.year = bcd_to_bin(t.year as u8) as u16;
	}

	if !is_24h && (t.hour & 0x80) != 0 {
		t.hour = ((t.hour & 0x7F) + 12) % 24;
	}	

	t.year += 2000;
	t
}

pub unsafe fn increment_one_second() {
    SECONDS += 1;
    if SECONDS < 60 {
        return;
    }
    SECONDS = 0;

    MINUTES += 1;
    if MINUTES < 60 {
        return;
    }
    MINUTES = 0;

    HOURS += 1;
    if HOURS < 24 {
        return;
    }
    HOURS = 0;

    DAY += 1;
    let dim = days_in_month(MONTH, YEAR);
    if DAY <= dim {
        return;
    }
    DAY = 1;

    MONTH += 1;
    if MONTH <= 12 {
        return;
    }
    MONTH = 1;

    YEAR += 1;
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(month: u8, year: u16) -> u8 {
    if month == 2 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[(month - 1) as usize]
    }
}


pub fn sleep(ms: u64) {
	let cycles = ms / 1000 * 3_000_000;
	unsafe {
		for _ in 0..cycles {
			asm!("pause");
		}
	}
}
