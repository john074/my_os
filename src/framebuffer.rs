use multiboot2::{ BootInformation, BootInformationHeader };

unsafe fn draw_red_screen(fb: &multiboot2::FramebufferTag) {
	let addr = fb.address() as *mut u32;
	let pixel_count = (fb.width() * fb.height()) as usize;
	for i in 0..pixel_count {	
		addr.add(i).write_volatile(0x00FF0000);
	}
}

pub fn init(multiboot_information_address: usize) {
	let boot_info = unsafe{ BootInformation::load(multiboot_information_address as *const BootInformationHeader).unwrap() };
	let fb_tag = boot_info.framebuffer_tag().expect("Framebuffer tag missing").unwrap();
	let fb_addr = fb_tag.address() as *mut u32;
	let pitch = fb_tag.pitch() as usize;
	let height = fb_tag.height() as usize;
	let width = fb_tag.width() as usize;
	unsafe {draw_red_screen(fb_tag) };
	    
	let framebuffer = unsafe {
	    core::slice::from_raw_parts_mut(fb_addr as *mut u8, pitch * height)
	};
		
	let mut put_pixel = |x: usize, y: usize, color: u32| {
	    if x >= width || y >= height { return; }
		let offset = y * pitch + x * 4; // 4 байта при 32bpp

		framebuffer[offset] = (color & 0xFF) as u8;     		 // B
		framebuffer[offset + 1] = ((color >> 8) & 0xFF) as u8;   // G  
		framebuffer[offset + 2] = ((color >> 16) & 0xFF) as u8;  // R
		framebuffer[offset + 3] = ((color >> 24) & 0xFF) as u8;  // A
	};
	
	let mut fill_rect = |x: usize, y: usize, w: usize, h: usize, color: u32| {
	    let xend = (x + w).min(width);
	    let yend = (y + h).min(height);
	    for yy in y..yend-1 {
	        for xx in x..xend-1 {
	            put_pixel(xx, yy, color);
	        }
	    }
	};
	
	let sq_w = 200usize;
	let sq_h = 200usize;
	let sq_x = (width.saturating_sub(sq_w)) / 2;
	let sq_y = (height.saturating_sub(sq_h)) / 2;
	
	let color: u32 = 0x2927f5cc;
	
	fill_rect(sq_x, sq_y, sq_w, sq_h, color);
}
