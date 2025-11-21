use alloc::vec::Vec;
use crate::framebuffer;
use crate::multitasking;
use crate::mouse;
use crate::println;

pub static mut GUI_PTR: *mut GuiSystem = core::ptr::null_mut();

pub enum GuiElement {
	Desktop(DesktopData),
    Window(WindowData),
    Button(ButtonData),
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
    	let win_id = self.add_node(self.root, GuiElement::Window(WindowData { title }), x, y, w, h);

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
	        framebuffer::FRAMEBUFFER.as_mut().unwrap().fill_rect(win.x, win.y, win.width, win.height, framebuffer::MAGENTA);
	    }
	}
}

#[allow(static_mut_refs)]
fn button_minimize(id: NodeId, gui: &mut GuiSystem) {
    let parent_id = gui.nodes[id].parent.unwrap();
    let window = &mut gui.nodes[parent_id];
    unsafe { framebuffer::FRAMEBUFFER.as_mut().unwrap().fill_screen(framebuffer::MAGENTA) };
    window.width = 200;
    window.height = 150;
    window.x = 50;
    window.y = 50;
    window.dirty = true;
    gui.mark_dirty(parent_id);
}

fn button_maximize(id: NodeId, gui: &mut GuiSystem) {
    let parent_id = gui.nodes[id].parent.unwrap();
    let window = &mut gui.nodes[parent_id];
    window.width = 1024;
    window.height = 768;
    window.x = 0;
    window.y = 0;
    window.dirty = true;
    gui.mark_dirty(parent_id);
}

fn button_close(id: NodeId, gui: &mut GuiSystem) {
    let parent_id = gui.nodes[id].parent.unwrap();
    gui.close_window(parent_id);
}


pub struct DesktopData {
}

pub struct WindowData {
    pub title: &'static str,
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

pub fn handle_mouse_down(x: isize, y: isize) {
	let gui = unsafe { &mut *GUI_PTR };
    if let Some(id) = gui.hit_test(gui.root, x, y) {
        match &mut gui.nodes[id].element {
            GuiElement::Button(btn) => {
            	btn.state = ButtonState::Pressed;
                gui.nodes[id].dirty = true;
            }
            GuiElement::Window(_) => {
            	// unsafe {
            	// 	(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(move_window(id)));
            	// }
            	let mouse = unsafe { &mut *mouse::MOUSE_PTR };
            	let window = &mut gui.nodes[id];
            	let dx = mouse.x - window.x;
            	let dy = mouse.y - window.y;	
            	gui.dragging = Some((id, dx, dy));
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

// #[allow(static_mut_refs)]
// pub async fn move_window(id: NodeId) {
// 	let mouse = unsafe { &mut *mouse::MOUSE_PTR };
// 	let gui = unsafe { &mut *GUI_PTR };
// 	let window = &mut gui.nodes[id];
// 	let dx = mouse.x - window.x;
// 	let dy = mouse.y - window.y;
// 	while mouse.l_pressed {
// 		unsafe { framebuffer::FRAMEBUFFER.as_mut().unwrap().fill_rect(window.x, window.y, window.width, window.height, framebuffer::MAGENTA) };
// 		window.x = mouse.x - dx;
// 		window.y = mouse.y - dy;
// 		window.dirty = true;
// 		unsafe { (*GUI_PTR).mark_dirty(id) };
// 		multitasking::cooperate().await
// 	}
// }
