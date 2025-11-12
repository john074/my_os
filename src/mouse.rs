use x86_64::instructions::port::{Port};
use crate::framebuffer;

pub static mut MOUSE_PTR: *mut Mouse = core::ptr::null_mut();
pub static mut MOUSE_X: isize = 512;
pub static mut MOUSE_Y: isize = 384;

const PS2_DATA: u16 = 0x60;
const PS2_CMD: u16 = 0x64;

fn wait_input_buffer_empty() {
    let mut status: u8;
    loop {
        unsafe {
            status = Port::<u8>::new(PS2_CMD).read();
        }
        if (status & 0x02) == 0 { break; }
    }
}

fn wait_output_buffer_full() {
    let mut status: u8;
    loop {
        unsafe {
            status = Port::<u8>::new(PS2_CMD).read();
        }
        if (status & 0x01) != 0 { break; }
    }
}

fn ps2_write_cmd(cmd: u8) {
    wait_input_buffer_empty();
    unsafe {
    	Port::<u8>::new(PS2_CMD).write(cmd);
    }
}

fn ps2_write_data(data: u8) {
    wait_input_buffer_empty();
    unsafe {
    	Port::<u8>::new(PS2_DATA).write(data);
    }
}

fn ps2_read_data() -> u8 {
    wait_output_buffer_full();
    unsafe {
    	Port::<u8>::new(PS2_DATA).read()
    }
}

fn pic_unmask_irq_master(bit: u8) {
    unsafe {
        let mut port = Port::<u8>::new(0x21);
        let mut mask = port.read();
        mask &= !(1 << bit);
        port.write(mask);
    }
}

fn pic_unmask_irq_slave(bit: u8) {
    unsafe {
        let mut port = Port::<u8>::new(0xA1);
        let mut mask = port.read();
        mask &= !(1 << bit);
        port.write(mask);
    }
}

pub fn init_mouse() -> Mouse {
    ps2_write_cmd(0xA8); // enable second port

    ps2_write_cmd(0x20); // read current cfg
    let mut cfg = ps2_read_data();

    // set bits: enable irq1 and irq12, ensure clk bits cleared, enable translator
    cfg |= 1 << 0;   // IRQ1 (keyboard) enable
    cfg |= 1 << 1;   // IRQ12 (mouse) enable
    cfg &= !(1 << 4); // keyboard clock enable (bit4=0)
    cfg &= !(1 << 5); // mouse clock enable (bit5=0)
    cfg |= 1 << 6;   // enable translation (bit6=1)

    ps2_write_cmd(0x60);
    ps2_write_data(cfg);

    ps2_write_cmd(0xD4);
    ps2_write_data(0xF4); // Enable data reporting
    let _ack = ps2_read_data(); // ACK (0xFA)

    pic_unmask_irq_master(1); // keyboard
    pic_unmask_irq_slave(4);  // mouse

    Mouse::new()
}

pub struct Mouse {
    pub x: isize,
    pub y: isize,
    pub prev_x: isize,
    pub prev_y: isize,
    pub width: usize,
    pub height: usize,
    pub saved_bg: [u32; 16*16],
}

impl Mouse {
    pub fn new() -> Self {
        Self {
            x: 100,
            y: 100,
            prev_x: 100,
            prev_y: 100,
            width: 16,
            height: 16,
            saved_bg: [0; 16*16],
        }
    }

    pub fn erase(&mut self, fb: &mut framebuffer::Framebuffer) {
        for yy in 0..self.height {
            for xx in 0..self.width {
                let color = self.saved_bg[yy * self.width + xx];
                fb.put_pixel_safe(self.prev_x + xx as isize, self.prev_y + yy as isize, color);
            }
        }
    }

    pub fn draw(&mut self, fb: &mut framebuffer::Framebuffer) {
        // save bg
        for yy in 0..self.height {
            for xx in 0..self.width {
                self.saved_bg[yy * self.width + xx] =
                    fb.get_pixel(self.x + xx as isize, self.y + yy as isize);
            }
        }

        fb.fill_triangle(
            self.x, self.y,
            self.x, self.y + 10,
            self.x + 10, self.y + 5,
            framebuffer::WHITE
        );

        self.prev_x = self.x;
        self.prev_y = self.y;
    }
}
