use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use core::{ pin::Pin, task::{ Poll, Context }};
use futures_util::stream::{ Stream, StreamExt };
use futures_util::task::AtomicWaker;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

use crate::println;
use crate::print;
use crate::vga_buffer;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

static INPUT_BUFFER: Mutex<Vec<char>> = Mutex::new(Vec::new());
pub static OUTPUT_STRING: Mutex<String> = Mutex::new(String::new());

pub fn add_scancode(scancode: u8) {
	if let Ok(queue) = SCANCODE_QUEUE.try_get() {
		if let Err(_) = queue.push(scancode) {
			println!("WARNING: scancode queue full; dropping keyboard input");
		} else {
			WAKER.wake();
		}
	} else {
		println!("WARNING: scancode queue uninitialized");
	}
}

pub struct ScancodeStream {
	private: (),
}

impl ScancodeStream {
	pub fn new() -> Self {
		SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100)).expect("ScancodeStream::new should only be called once");
		ScancodeStream { private:() }
	}
}

impl Stream for ScancodeStream {
	type Item = u8;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
		let queue = SCANCODE_QUEUE.try_get().expect("not initialized");

		if let Some(scancode) = queue.pop() {
			return Poll::Ready(Some(scancode));
		}

		WAKER.register(&cx.waker());
		match queue.pop() {
			Some(scancode) => {
				WAKER.take();
				Poll::Ready(Some(scancode))
			}
			None => Poll::Pending
		}
	}
}

pub async fn print_keypresses() {
	let mut scancodes = ScancodeStream::new();
	let mut keyboard = Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore);

	while let Some(scancode) = scancodes.next().await {
		if scancode == 0x0E {
			vga_buffer::rm_char();
			INPUT_BUFFER.lock().pop();
		}
		else if scancode == 0x1C {
			print!("\n>");
			let new_str: alloc::string::String = INPUT_BUFFER.lock().iter().collect();
			*OUTPUT_STRING.lock() = new_str;
			INPUT_BUFFER.lock().clear();
		}
		else {
			if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
				if let Some(key) = keyboard.process_keyevent(key_event) {
					match key {
						DecodedKey::Unicode(character) => {
							print!("{}", character);
							INPUT_BUFFER.lock().push(character);	
						},
						DecodedKey::RawKey(key) => print!(""),
					}
				}
			}
		}
	}
}
