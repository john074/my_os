use core::fmt::{self, Write};
use crate::std::syscall;

struct SyscallWriter;

impl Write for SyscallWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        syscall(1, s.as_ptr() as u64, s.len() as u64, 0, 0);
        Ok(())
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
