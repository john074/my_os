#![no_std]
#![no_main]

extern crate alloc;

use TEST::std::{ syscall, exit, sleep, multitasking };
use TEST::{ print, println };

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    let mut task = multitasking::Task::new(user(), Some(5));
    syscall::spawn_task((&mut task as *mut multitasking::Task) as u64);
    exit();
}

async fn user() {
    for i in 0..10 {
        println!("B");
        multitasking::cooperate().await;
        sleep(2000);
    }
    exit();
}
