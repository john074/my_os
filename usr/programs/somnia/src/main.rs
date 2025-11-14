#![no_std]
#![no_main]

extern crate alloc;

use somnia::std::{ multitasking, syscall, exit };
use somnia::{ print, println };
use alloc::vec::Vec;
use alloc::format;
use alloc::string::{ ToString, String };

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    let mut task = multitasking::Task::new(user_main());
    syscall::spawn_task((&mut task as *mut multitasking::Task) as u64);
    exit();
}


async fn user_main() {
    let txt = "SOMNIA shell 0.1";
    println!("{}", txt);
    print!(">");
    let mut buf: Vec<char> = Vec::new();
    let mut current_dir: String  = "/".to_string();
	let mut input: String = "".to_string();
	let mut current_command_buffer: Vec<char> = Vec::new();

	current_command_buffer.push('>');
    
    loop {
    	somnia::std::read((&mut buf as *mut Vec<char>) as u64);
    	multitasking::cooperate().await;

		for i in buf.clone() {
			if i == '\n' {
				println!("");
				print!(">");
				current_command_buffer.remove(0);
				input = current_command_buffer.iter().collect();
				current_command_buffer.clear();
				current_command_buffer.push('>');
			}
			else if i == '\x08' {
				if current_command_buffer.len() == 1 {
					continue;
				}
				current_command_buffer.pop();
				somnia::std::rm_char();
			}
			else {
				current_command_buffer.push(i);
				print!("{}", i);
			}
		}

		buf.clear();
		if input == "" {
			continue;
		}

		let parts = input.split(" ").collect::<Vec<&str>>();

    	match &parts[0] {
    		&"pwd" => { 
    			println!("{}", current_dir);
    			print!(">");
    		}
    		
    		&"cd" => {
    			if parts.len() < 2 {
    				current_dir = "/".to_string();
    				input = "".to_string();
    				continue
    			}
    			
    			let mut new_path = parse_path(&current_dir, &mut parts[1].to_string());
				let mut cd = new_path.clone();
    			if cd.contains("..") {
    				new_path = normalize_path(&cd);
    			}
    			
    			if somnia::std::check_fs_entry_exists(&new_path) == 1 || &new_path == "/"{
  		  			current_dir = new_path;
    			}
    			else {
    				println!("path does not exist");
    				print!(">");
    			}
    		},
    		
    		&"ls" => {
    			let mut dir_contents:Vec<String> = Vec::new();
    			somnia::std::ls(&current_dir, (&mut dir_contents as *mut Vec<String>) as u64);
    			for i in dir_contents {
    				print!("{}  ", i);
    			}
    			println!("");
    			print!(">");
    		},

    		&"mkdir" | &"mkfile" => {
   				if parts.len() < 2 {
    				println!("specify dir/file name like 'mkdir my_dir' or mkfile 'my_file'");
    				print!(">");
    				input = "".to_string();
    				continue
    			}
    			
    			let mut new_path = parse_path(&current_dir, &mut parts[1].to_string());
    			if somnia::std::check_fs_entry_exists(&new_path) == 0 {
    				if parts[0] == "mkdir" {
    					somnia::std::mkdir(&new_path);
    				}
    				else {
    					somnia::std::mkfile(&new_path);
    				}
    			}
    			else {
    				println!("dir or file with such name already exists");
    				print!(">");
    			}
    		},

    		&"rmdir" | &"rmfile" => {
   				if parts.len() < 2 {
    				println!("specify dir/file name like 'rmdir my_dir' or rmfile 'my_file'");
    				print!(">");
    				input = "".to_string();
    				continue
    			}
    			
    			let mut path = parse_path(&current_dir, &mut parts[1].to_string());
    			if somnia::std::check_fs_entry_exists(&path) == 1 {
    				if parts[0] == "rmdir" {
    					somnia::std::rmdir(&path);
    				}
    				else {
    					somnia::std::rmfile(&path);
    				}
    			}
    			else {
    				println!("dir or file with such name does not exist");
    				print!(">");
    			}
    		},

    		&"write" => {
  				if parts.len() < 3 {
    				println!("specify dest file and data like 'write my_file.txt \"My very important piece of data!\"'");
    				print!(">");
    				input = "".to_string();
    				continue
    			}
    			if input.matches('"').count() != 2 {
    				println!("data must be surrounded by '\"' and not contain it inside. Like \"My data\"");
    				print!(">");
    				input = "".to_string();
    				continue
    			}
    			
				let mut new_path = parse_path(&current_dir, &mut parts[1].to_string());
    			if somnia::std::check_fs_entry_exists(&new_path) == 0 {
    				somnia::std::mkfile(&new_path);
    			}

    			let mut content = input.split("\"").collect::<Vec<&str>>()[1];
    			somnia::std::write_file(&new_path, &content);
    		},

    		&"read" => {
    			if parts.len() < 2 {
    				println!("specify dest file 'read my_file.txt'");
    				print!(">");
    				input = "".to_string();
    				continue
    			}

    			let mut new_path = parse_path(&current_dir, &mut parts[1].to_string());
				if somnia::std::check_fs_entry_exists(&new_path) == 1 {
					let mut buffer = [0u8; 2024];
					let buff_ptr = buffer.as_mut_ptr() as u64;
					let len = somnia::std::read_file(&new_path, buff_ptr, buffer.len() as u64);
					println!("{}", core::str::from_utf8(&buffer[..len as usize]).unwrap_or("[invalid utf8]"));
    				print!(">");
    			}
    			else {
    				println!("file not found");
    				print!(">");
    			}
    		}

    		&"clear" => {
				somnia::std::clear_screen();
    			print!(">");
    		},

    		&"run" => {
   				if parts.len() < 2 {
    				println!("specify file name like 'run my_file'");
    				print!(">");
    				input = "".to_string();
    				continue
    			}
    			
    			let mut path = parse_path(&current_dir, &mut parts[1].to_string());
    			if somnia::std::check_fs_entry_exists(&path) == 1 {
    				somnia::std::run(&path);
    			}
    			else {
    				println!("file with such name does not exist");
    				print!(">");
    			}
    		},
    		
  			_ => { 
  				println!("Unknown command: {}", input);
  				print!(">");
  			}
    	}

    	input = "".to_string();
    }
    exit();
}

fn parse_path(current_dir: &str, name: &str) -> String {
    let trimmed = name.trim_end_matches('/');
    normalize_path(&format!("{}/{}", current_dir, trimmed))
}

fn normalize_path(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    let mut is_absolute = path.starts_with('/');
    
    for component in path.split('/') {
        match component {
            "" | "." => continue,
            ".." => {
                if !stack.is_empty() {
                    stack.pop();
                } else if !is_absolute {
                    stack.push("..");
                }
            },
            part => stack.push(part),
        }
    }
    
    let mut result = String::new();
    
    if is_absolute {
        result.push('/');
    } else if stack.is_empty() {
        return ".".to_string();
    }
    
    result.push_str(&stack.join("/"));
    
    if result == "/" {
        return result;
    }
    
    if result.ends_with('/') {
        result.pop();
    }
    
    result
}
