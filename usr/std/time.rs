pub fn sleep(ms: u64) {
    let cycles = ms / 1000 * 3_000_000;
    unsafe {
        for _ in 0..cycles {
            core::arch::asm!("pause");
        }
    }
}
