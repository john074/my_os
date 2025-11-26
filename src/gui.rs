use alloc::vec::Vec;
use crate::framebuffer;
use crate::mouse;
use alloc::string::String;

pub static mut GUI_PTR: *mut GuiSystem = core::ptr::null_mut();

// GUI ELEMENTS

pub enum GuiElement {
	Desktop(DesktopData),
    Window(WindowData),
    Button(ButtonData),
    Terminal(TerminalData),
    Dead,
}

pub type NodeId = usize;

pub struct GuiNode {
    pub element: GuiElement,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub x: isize,
    pub y: isize,
    pub width: isize,
    pub height: isize,
    pub dirty: bool,
}

pub struct GuiSystem {
    pub nodes: Vec<GuiNode>,
    pub root: NodeId, // desktop
    pub free_list: Vec<NodeId>,
   	pub dragging: Option<(NodeId, isize, isize)>,
   	pub resizing: Option<NodeId>,
}

impl GuiSystem {
    pub fn new(fb_width: isize, fb_height: isize) -> Self {
        let root = GuiNode {
            element: GuiElement::Desktop(DesktopData {}),
            parent: None,
            children: Vec::new(),
            x: 0,
            y: 0,
            width: fb_width,
            height: fb_height,
            dirty: true,
        };

        Self {
            nodes: vec![root],
            root: 0,
            free_list: Vec::new(),
            dragging: None,
            resizing: None,
        }
    }

    pub fn add_node(&mut self, parent: NodeId, element: GuiElement, x: isize, y: isize, w: isize, h: isize) -> NodeId {
		let id = if let Some(free_id) = self.free_list.pop() {
		    self.nodes[free_id] = GuiNode { element, parent: Some(parent), children: Vec::new(), x, y, width: w, height: h, dirty: true };
		    free_id
		} else {
		    let id = self.nodes.len();
			self.nodes.push(GuiNode {element, parent: Some(parent), children: Vec::new(), x, y, width: w, height: h, dirty: true});
		    id
		};
        self.nodes[parent].children.push(id);
        id
    }

    pub fn draw(&mut self, fb: &mut framebuffer::Framebuffer) {
        self.draw_node(self.root, 0, 0, fb);
    }

	fn draw_node(&mut self, id: NodeId, ox: isize, oy: isize, fb: &mut framebuffer::Framebuffer) {
		self.adjust_safe_placement(id);
	    let (ax, ay, children, dirty, element) = {
	        let node = &self.nodes[id];
	        (
	            ox + node.x,
	            oy + node.y,
	            node.children.clone(),
	            node.dirty,
	            &node.element
	        )
	    };

	    if dirty {
	        match element {
	            GuiElement::Window(w) => draw_window(ax, ay, self.nodes[id].width, self.nodes[id].height, w, fb),
	            GuiElement::Button(b) => draw_button(ax, ay, self.nodes[id].width, self.nodes[id].height, b, fb),
	            GuiElement::Terminal(t) => draw_terminal(ax, ay, self.nodes[id].width, self.nodes[id].height, t, fb),
	            GuiElement::Desktop(_) => {}
	            GuiElement::Dead => {}
	        }

	        self.nodes[id].dirty = false;
	    }

	    for &child in &children {
	        self.draw_node(child, ax, ay, fb);
	    }

	}

	pub fn create_window(&mut self, title: &'static str, x: isize, y: isize, w: isize, h: isize) -> NodeId {
    	let win_id = self.add_node(self.root, GuiElement::Window(WindowData {
    	 title,
    	 minimized_x: x,
    	 minimized_y: y,
    	 minimized_width: w, 
    	 minimized_height: h }), x, y, w, h);

        let bw = 20;
        let bh = 18;

        let close_x = w - bw - 4;
        let max_x = close_x - bw - 2;
        let min_x = max_x - bw - 2;
        let by = 3;

        self.add_node(win_id, GuiElement::Button(ButtonData::new("-", Some(button_minimize))), min_x, by, bw, bh);
        self.add_node(win_id, GuiElement::Button(ButtonData::new("+", Some(button_maximize))), max_x, by, bw, bh);
        self.add_node(win_id, GuiElement::Button(ButtonData::new("X", Some(button_close))), close_x, by, bw, bh);

        win_id
	}

    pub fn hit_test(&self, id: NodeId, x: isize, y: isize) -> Option<NodeId> {
         let node = &self.nodes[id];

         if x < node.x || y < node.y || x >= node.x + node.width || y >= node.y + node.height {
             return None;
         }

         let lx = x - node.x;
         let ly = y - node.y;

         for &child_id in node.children.iter().rev() {
             if let Some(hit) = self.hit_test(child_id, lx, ly) {
                 return Some(hit);
             }
         }

         Some(id)
	}

	pub fn mark_dirty(&mut self, id: NodeId) {
	    let children = self.nodes[id].children.clone();
	    
	    for &child_id in &children {
	        self.nodes[child_id].dirty = true;
	    }
	}

	pub fn kill_node(&mut self, id: NodeId) {
		let node = &mut self.nodes[id];
		node.element = GuiElement::Dead;
		node.children.clear();
		self.free_list.push(id);
	}

	pub fn delete_subtree(&mut self, id: NodeId) {
		let children = self.nodes[id].children.clone();
		for child in children {
		    self.delete_subtree(child);
		}
		self.kill_node(id);
	}

	#[allow(static_mut_refs)]
	pub fn close_window(&mut self, window_id: NodeId) {
	    self.delete_subtree(window_id);
	
	    if let Some(parent) = self.nodes[window_id].parent {
	        self.nodes[parent].children.retain(|&c| c != window_id);
	    }
	
	    let win = &self.nodes[window_id];
	    unsafe {
	        framebuffer::FRAMEBUFFER.as_mut().unwrap().blit_rect_from_wallpaper(win.x, win.y, win.width as usize, win.height as usize);
	    }
	}

	pub fn adjust_safe_placement(&mut self, id: NodeId) {
		let window = &mut self.nodes[id];
	
		if let GuiElement::Window(_) = window.element {
			if window.x < 0 { window.x = 0; }
			if window.y < 0 { window.y = 0; }
			if window.x > 933 { window.x = 934; }
			if window.y > 743 { window.y = 743; }
			if window.width > 1024 { window.width = 1024; }
			if window.height > 768 { window.height = 768; }
			if window.x + window.width > 1023 { window.x = 1024 - window.width; }
			if window.y + window.height > 767 { window.y = 768 - window.height; }
		}
	}

	pub fn bring_to_front(&mut self, id: NodeId) {
	    let parent = self.nodes[id].parent.unwrap();
	    let children = &mut self.nodes[parent].children;

	    if let Some(pos) = children.iter().position(|&c| c == id) {
	        let item = children.remove(pos);
	        children.push(item);
	    }

	    self.nodes[id].dirty = true;
	}

	pub fn mark_overlapping_windows_dirty(&mut self, win_id: NodeId) {
	    let (x, y, w, h) = {
	        let w = &self.nodes[win_id];
	        (w.x, w.y, w.width, w.height)
	    };

	    let siblings = self.nodes[self.root].children.clone();

	    for &other in &siblings {
	        if other == win_id { continue; }
	        let n = &self.nodes[other];

	        if rect_intersects(x,y,w,h, n.x,n.y,n.width,n.height) {
	            self.nodes[other].dirty = true;
	            self.mark_dirty(other);
	        }
	    }
	}
}

// DATA STRUCTS

pub struct DesktopData {
}

pub struct WindowData {
    pub title: &'static str,
    pub minimized_x: isize,
    pub minimized_y: isize,
    pub minimized_width: isize,
    pub minimized_height: isize,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Pressed,
}

pub struct ButtonData {
    pub text: &'static str,
    pub on_click: Option<fn(NodeId, &mut GuiSystem)>,
    pub state: ButtonState,
}


impl ButtonData {
    pub fn new(text: &'static str, on_click: Option<fn(NodeId, &mut GuiSystem)>) -> Self {
        Self {
            text,
            on_click,
            state: ButtonState::Normal
        }
    }
}

pub struct TerminalData {
    pub buffer: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub text_color: u32,
}

// WINDOW HELP FUNCTIONS

fn button_maximize(id: NodeId, gui: &mut GuiSystem) {
	let parent_id = gui.nodes[id].parent.unwrap();
	gui.bring_to_front(parent_id);
	let window = &mut gui.nodes[parent_id];
	
	if let GuiElement::Window(window_data) = &mut window.element {
		window_data.minimized_x = window.x;
		window_data.minimized_y = window.y;
		window_data.minimized_width = window.width;
		window_data.minimized_height = window.height;
	}

	window.x = 0;
	window.y = 0;
	resize_window(parent_id, 1024 - window.width, 768 - window.height);

	let siblings = gui.nodes[gui.root].children.clone();
	for &other in &siblings {
	    if other == id { continue; }
	    gui.nodes[other].dirty = true;
	    gui.mark_dirty(other);
	}
}

fn button_minimize(id: NodeId, gui: &mut GuiSystem) {
    let parent_id = gui.nodes[id].parent.unwrap();
    gui.bring_to_front(parent_id);
    let window = &mut gui.nodes[parent_id];
    
    if let GuiElement::Window(window_data) = &window.element {
		resize_window(parent_id, window_data.minimized_width - window.width, window_data.minimized_height - window.height);
		window.x = window_data.minimized_x;
		window.y = window_data.minimized_y;
    } 
    
    let siblings = gui.nodes[gui.root].children.clone();
    for &other in &siblings {
        if other == id { continue; }
        gui.nodes[other].dirty = true;
        gui.mark_dirty(other);
    }  
}

fn button_close(id: NodeId, gui: &mut GuiSystem) {
    let parent_id = gui.nodes[id].parent.unwrap();
    gui.close_window(parent_id);
}

#[allow(static_mut_refs)]
pub fn resize_window(id: NodeId, dx: isize, dy: isize) {
	let gui = unsafe { &mut *GUI_PTR };

	let window_width = {
		let window = &mut gui.nodes[id];

		if dx < 0 || dy < 0 {
			unsafe {
			    framebuffer::FRAMEBUFFER.as_mut().unwrap().blit_rect_from_wallpaper(window.x, window.y, window.width as usize, window.height as usize);
			}
		}
		
		if window.width + dx > 1024 {
			window.width = 1024;
		}
		else if window.width + dx < 90 {
			window.width = 90;
		}
		else {
			window.width += dx;
		}
		
		if window.height + dy > 768 {
			window.height = 768;
		}
		else if window.height + dy < 24 {
			window.height = 24;
		}
		else {
			window.height += dy;
		}

		window.dirty = true;
		window.width
	};

	let bw = 20;

	let cls_x = {
		let cls_btn = &mut gui.nodes[id + 3];
		cls_btn.x = window_width - bw - 4;
		cls_btn.x
	};

	let max_x = {
		let max_btn = &mut gui.nodes[id + 2];
		max_btn.x = cls_x - bw - 2;
		max_btn.x
	};

	let min_btn = &mut gui.nodes[id + 1];
    min_btn.x = max_x - bw - 2;

    gui.mark_dirty(id);
}

fn rect_intersects(a_x: isize, a_y: isize, a_w: isize, a_h: isize, b_x: isize, b_y: isize, b_w: isize, b_h: isize) -> bool {
    !(a_x + a_w <= b_x ||
      b_x + b_w <= a_x ||
      a_y + a_h <= b_y ||
      b_y + b_h <= a_y)
}

// DRAWING

const WIN95_BG: u32 = 0xC0C0C0;
const WIN95_TITLE_BAR: u32 = 0x000080;
const WIN95_TITLE_TEXT: u32 = 0xFFFFFF;
const WIN95_SHADOW_DARK: u32 = 0x606060;
const WIN95_SHADOW_LIGHT: u32 = 0xFFFFFF;
const WIN95_BUTTON_BG: u32 = 0xC0C0C0;
const WIN95_BUTTON_BORDER: u32 = 0x000000;

fn draw_window(x: isize, y: isize, w: isize, h: isize, data: &WindowData, fb: &mut framebuffer::Framebuffer) {
    fb.fill_rect(x, y, w, h, WIN95_BG); // body
    fb.draw_rect(x, y, w, h, WIN95_SHADOW_DARK); // borders

    let title_h = 24;
    fb.fill_rect(x + 1, y + 1, w - 2, title_h, WIN95_TITLE_BAR); // title
    fb.draw_string(x + 5, y + 5, data.title, WIN95_TITLE_TEXT); // title text
}

pub fn draw_button(x: isize, y: isize, w: isize, h:isize, b:&ButtonData, fb: &mut framebuffer::Framebuffer) {
    if b.state == ButtonState::Pressed {
        fb.fill_rect(x+1, y+1, w, h, WIN95_BUTTON_BG);
        fb.draw_rect(x+1, y+1, w, h, WIN95_SHADOW_DARK);
        fb.draw_string(x + 6 + 1, y + 1 + 1, b.text, 0x000000);
    } else {
        fb.fill_rect(x, y, w, h, WIN95_BUTTON_BG);
        fb.draw_rect(x, y, w, h, WIN95_BUTTON_BORDER);
        fb.draw_string(x + 6, y + 1, b.text, 0x000000);
    }
}

fn draw_terminal(x: isize, y: isize, w: isize, h: isize, term: &TerminalData, fb: &mut framebuffer::Framebuffer) {
    fb.fill_rect(x, y, w, h, 0x000000);

    let mut cy = y;
    for line in &term.buffer {
        fb.draw_string(x + 2, cy, line, term.text_color);
        cy += 16;
        if cy >= y + h {
            break;
        }
    }
}


// MOUSE HADLING

pub fn handle_mouse_down(x: isize, y: isize) {
	let gui = unsafe { &mut *GUI_PTR };
    if let Some(id) = gui.hit_test(gui.root, x, y) {
        match &mut gui.nodes[id].element {
            GuiElement::Button(btn) => {
            	btn.state = ButtonState::Pressed;
                gui.nodes[id].dirty = true;
            }
            GuiElement::Window(_) => {
            	gui.bring_to_front(id);
            	let mouse = unsafe { &mut *mouse::MOUSE_PTR };
            	let window = &mut gui.nodes[id];

				if (window.x + window.width - mouse.x <= 10) || (window.y + window.height - mouse.y <= 10) {
					gui.resizing = Some(id);
				}
            	else {
	            	let dx = mouse.x - window.x;
	            	let dy = mouse.y - window.y;	
	            	gui.dragging = Some((id, dx, dy));
	            }
            }
            _ => {}
        }
    }
}

pub fn handle_mouse_up(x: isize, y: isize) {
	let gui = unsafe { &mut *GUI_PTR };
    if let Some(id) = gui.hit_test(gui.root, x, y) {
        match &mut gui.nodes[id].element {
            GuiElement::Button(btn) => {
                if let Some(action) = btn.on_click {
                    btn.state = ButtonState::Normal;
                    action(id, gui);
                }
                gui.nodes[id].dirty = true;
            }
            _ => {}
        }
    }
}
