const VGA_PORT_COMMAND: u16 = 0x3D4;
const VGA_PORT_DATA: u16 = 0x3D5;

pub fn set_cursor_position(position: u16) {
    unsafe {
        use x86_64::instructions::port::Port;

        let mut command_port = Port::new(VGA_PORT_COMMAND);
        let mut data_port = Port::new(VGA_PORT_DATA);

        command_port.write(0x0F_u8);
        data_port.write((position & 0xFF) as u8);
        command_port.write(0x0E_u8);
        data_port.write((position >> 8) as u8);
    }
}

pub fn disable_cursor() {
    unsafe {
        use x86_64::instructions::port::Port;
            
        let mut command_port = Port::new(VGA_PORT_COMMAND);
        let mut data_port = Port::new(VGA_PORT_DATA);

        command_port.write(0x0A as u32);
        data_port.write(0x20 as u32);
    }
}

