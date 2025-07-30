use core::fmt::{self, Write};
use crate::std::syscall;
use crate::std::multitasking;
use crate::println;

struct SyscallWriter;

impl Write for SyscallWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        syscall(1, s.as_ptr() as u64, s.len() as u64, 0);
        Ok(())
    }
}

pub async fn console_read(buf: &mut [u8]) -> &str {
	let buff_ptr = buf.as_mut_ptr() as u64;
	let last_command = {
		let len = syscall::read(buff_ptr, buf.len() as u64);
		core::str::from_utf8(&buf[..len as usize]).unwrap_or("[invalid utf8]")
	};


	loop {
		let command = {
			let len = syscall::read(buff_ptr, buf.len() as u64);
			core::str::from_utf8(&buf[..len as usize]).unwrap_or("[invalid utf8]")
		};

		if command != last_command {
			return command
		}

		multitasking::cooperate().await;
	}
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let _ = SyscallWriter.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::std::io::_print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
