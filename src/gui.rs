use alloc::vec::Vec;
use crate::framebuffer::Framebuffer;

pub static mut GUI_PTR: *mut GuiSystem = core::ptr::null_mut();

pub enum GuiElement {
	Desktop(DesktopData),
    Window(WindowData),
}

pub struct DesktopData {
}

pub struct WindowData {
    pub title: &'static str,
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
        }
    }

    pub fn add_node(&mut self, parent: NodeId, element: GuiElement, x: isize, y: isize, w: isize, h: isize) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(GuiNode {element, parent: Some(parent), children: Vec::new(), x, y, width: w, height: h, dirty: true});
        self.nodes[parent].children.push(id);
        id
    }

    pub fn draw(&mut self, fb: &mut Framebuffer) {
        self.draw_node(self.root, 0, 0, fb);
    }

    fn draw_node(&mut self, id: NodeId, ox: isize, oy: isize, fb: &mut Framebuffer) {
        let (ax, ay, children) = {
            let node = &mut self.nodes[id];
            
            let ax = ox + node.x;
            let ay = oy + node.y;
    
            if node.dirty {
                match &node.element {
                    GuiElement::Window(w) => {
                        draw_window(ax, ay, node.width, node.height, w, fb);
                        fb.mark_dirty(ax, ay, node.width, node.height);
                    }
                    GuiElement::Desktop(_) => {}
                }
                node.dirty = false;
            }
    
            (ax, ay, node.children.clone())
        };
    
        for &child in &children {
            self.draw_node(child, ax, ay, fb);
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

fn draw_window(x: isize, y: isize, w: isize, h: isize, data: &WindowData, fb: &mut Framebuffer) {
    fb.fill_rect(x, y, w, h, WIN95_BG); // body
    fb.draw_rect(x, y, w, h, WIN95_SHADOW_DARK); // borders
    
    let title_h = 24;
    fb.fill_rect(x + 1, y + 1, w - 2, title_h, WIN95_TITLE_BAR); // title
    fb.draw_string(x + 5, y + 5, data.title, WIN95_TITLE_TEXT); // title text

    // bts
    let btn_w = 20;
    let btn_h = 18;

    let close_x = x + w - btn_w - 4;
    let max_x = close_x - btn_w - 2;
    let min_x = max_x - btn_w - 2;
    let btn_y = y + 3;

    draw_win95_button(min_x, btn_y, btn_w, btn_h, "-", fb);
    draw_win95_button(max_x, btn_y, btn_w, btn_h, "+", fb);
    draw_win95_button(close_x, btn_y, btn_w, btn_h, "X", fb);
}

fn draw_win95_button(x: isize, y: isize, w: isize, h: isize, label: &str, fb: &mut Framebuffer) {
    fb.fill_rect(x, y, w, h, WIN95_BUTTON_BG);
    fb.draw_rect(x, y, w, h, WIN95_BUTTON_BORDER);
    fb.draw_string(x + 6, y+1, label, 0x000000);
}
