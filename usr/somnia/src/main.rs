#![no_std]
#![no_main]

extern crate alloc;

use somnia::std::{ syscall, exit, sleep, console_read, multitasking };
use somnia::{ print, println };

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    let mut task = multitasking::Task::new(user());
    syscall::spawn_task((&mut task as *mut multitasking::Task) as u64);
}

async fn user() {
    let txt = "SOMNIA shell 0.1";
    println!("{}", txt);
    let mut buf = [0u8; 128];
    let supported_commands = ["ls", "cd", "mkdir", "mkfile"];
    let mut current_dir = "/";
    
    loop {
    	let input = console_read(&mut buf).await;
    	if !supported_commands.contains(&input) {
    		println!("Unknown command: {}", input);	
    	}
    }
    exit();
}
