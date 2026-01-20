// DA CUBE
const SCALE: isize = 256;
const FP: isize = 1024;

#[derive(Clone, Copy)]
pub struct Point {
    pub x: isize,
    pub y: isize,
    pub z: isize,
}

pub const VS: [Point; 176] = [
	// S
    Point{x: -260, y:  40, z:  16},
    Point{x: -340, y:  40, z:  16},
    Point{x: -340, y:  20, z:  16},
    Point{x: -260, y:  20, z:  16},
    Point{x: -260, y:  40, z: -16},
    Point{x: -340, y:  40, z: -16},
    Point{x: -340, y:  20, z: -16},
    Point{x: -260, y:  20, z: -16},

    Point{x: -340, y:  20, z:  16},
    Point{x: -320, y:  20, z:  16},
    Point{x: -320, y:   0, z:  16},
    Point{x: -340, y:   0, z:  16},
    Point{x: -340, y:  20, z: -16},
    Point{x: -320, y:  20, z: -16},
    Point{x: -320, y:   0, z: -16},
    Point{x: -340, y:   0, z: -16},

    Point{x: -340, y:   0, z:  16},
    Point{x: -260, y:   0, z:  16},
    Point{x: -260, y: -20, z:  16},
    Point{x: -340, y: -20, z:  16},
    Point{x: -340, y:   0, z: -16},
    Point{x: -260, y:   0, z: -16},
    Point{x: -260, y: -20, z: -16},
    Point{x: -340, y: -20, z: -16},

    Point{x: -280, y: -20, z:  16},
    Point{x: -260, y: -20, z:  16},
    Point{x: -260, y: -40, z:  16},
    Point{x: -280, y: -40, z:  16},
    Point{x: -280, y: -20, z: -16},
    Point{x: -260, y: -20, z: -16},
    Point{x: -260, y: -40, z: -16},
    Point{x: -280, y: -40, z: -16},

    Point{x: -340, y: -40, z:  16},
    Point{x: -260, y: -40, z:  16},
    Point{x: -260, y: -60, z:  16},
    Point{x: -340, y: -60, z:  16},
    Point{x: -340, y: -40, z: -16},
    Point{x: -260, y: -40, z: -16},
    Point{x: -260, y: -60, z: -16},
    Point{x: -340, y: -60, z: -16},

	// O
    Point{x: -200, y:  40, z:  16},
    Point{x: -180, y:  40, z:  16},
    Point{x: -180, y: -60, z:  16},
    Point{x: -200, y: -60, z:  16},
    Point{x: -200, y:  40, z: -16},
    Point{x: -180, y:  40, z: -16},
    Point{x: -180, y: -60, z: -16},
    Point{x: -200, y: -60, z: -16},

    Point{x: -180, y:  40, z:  16},
    Point{x: -100, y:  40, z:  16},
    Point{x: -100, y:  20, z:  16},
    Point{x: -180, y:  20, z:  16},
    Point{x: -180, y:  40, z: -16},
    Point{x: -100, y:  40, z: -16},
    Point{x: -100, y:  20, z: -16},
    Point{x: -180, y:  20, z: -16},

    Point{x: -120, y:  20, z:  16},
    Point{x: -100, y:  20, z:  16},
    Point{x: -100, y: -40, z:  16},
    Point{x: -120, y: -40, z:  16},
    Point{x: -120, y:  20, z: -16},
    Point{x: -100, y:  20, z: -16},
    Point{x: -100, y: -40, z: -16},
    Point{x: -120, y: -40, z: -16},

    Point{x: -200, y: -40, z:  16},
    Point{x: -100, y: -40, z:  16},
    Point{x: -100, y: -60, z:  16},
    Point{x: -200, y: -60, z:  16},
    Point{x: -200, y: -40, z: -16},
    Point{x: -100, y: -40, z: -16},
    Point{x: -100, y: -60, z: -16},
    Point{x: -200, y: -60, z: -16},

	// M
    Point{x: -60, y:  40, z:  16},
    Point{x: -40, y:  40, z:  16},
    Point{x: -40, y: -60, z:  16},
    Point{x: -60, y: -60, z:  16},
    Point{x: -60, y:  40, z: -16},
    Point{x: -40, y:  40, z: -16},
    Point{x: -40, y: -60, z: -16},
    Point{x: -60, y: -60, z: -16},

    Point{x:  20, y:  40, z:  16},
    Point{x:  40, y:  40, z:  16},
    Point{x:  40, y: -60, z:  16},
    Point{x:  20, y: -60, z:  16},
    Point{x:  20, y:  40, z: -16},
    Point{x:  40, y:  40, z: -16},
    Point{x:  40, y: -60, z: -16},
    Point{x:  20, y: -60, z: -16},

    Point{x: -40, y:  40, z:  16},
    Point{x: -20, y:  40, z:  16},
    Point{x: -20, y: -20, z:  16},
    Point{x: -40, y: -20, z:  16},
    Point{x: -40, y:  40, z: -16},
    Point{x: -20, y:  40, z: -16},
    Point{x: -20, y: -20, z: -16},
    Point{x: -40, y: -20, z: -16},

    Point{x:   0, y:  40, z:  16},
    Point{x:  20, y:  40, z:  16},
    Point{x:  20, y: -20, z:  16},
    Point{x:   0, y: -20, z:  16},
    Point{x:   0, y:  40, z: -16},
    Point{x:  20, y:  40, z: -16},
    Point{x:  20, y: -20, z: -16},
    Point{x:   0, y: -20, z: -16},

    Point{x: -20, y: -20, z:  16},
    Point{x:   0, y: -20, z:  16},
    Point{x:   0, y: -40, z:  16},
    Point{x: -20, y: -40, z:  16},
    Point{x: -20, y: -20, z: -16},
    Point{x:   0, y: -20, z: -16},
    Point{x:   0, y: -40, z: -16},
    Point{x: -20, y: -40, z: -16},

	//N
    Point{x:  80, y:  40, z:  16},
    Point{x:  100, y:  40, z:  16},
    Point{x:  100, y: -60, z:  16},
    Point{x:  80, y: -60, z:  16},
    Point{x:  80, y:  40, z: -16},
    Point{x:  100, y:  40, z: -16},
    Point{x:  100, y: -60, z: -16},
    Point{x:  80, y: -60, z: -16},

    Point{x:  80, y:  40, z:  16},
    Point{x: 100, y:  40, z:  16},
    Point{x: 180, y: -60, z:  16},
    Point{x: 160, y: -60, z:  16},
    Point{x:  80, y:  40, z: -16},
    Point{x: 100, y:  40, z: -16},
    Point{x: 180, y: -60, z: -16},
    Point{x: 160, y: -60, z: -16},

    Point{x: 160, y:  40, z:  16},
    Point{x: 180, y:  40, z:  16},
    Point{x: 180, y: -60, z:  16},
    Point{x: 160, y: -60, z:  16},
    Point{x: 160, y:  40, z: -16},
    Point{x: 180, y:  40, z: -16},
    Point{x: 180, y: -60, z: -16},
    Point{x: 160, y: -60, z: -16},

	// I
    Point{x: 220, y:  40, z:  16},
    Point{x: 240, y:  40, z:  16},
    Point{x: 240, y: -60, z:  16},
    Point{x: 220, y: -60, z:  16},
    Point{x: 220, y:  40, z: -16},
    Point{x: 240, y:  40, z: -16},
    Point{x: 240, y: -60, z: -16},
    Point{x: 220, y: -60, z: -16},

	// A
    Point{x: 280, y:  20, z:  16},
    Point{x: 300, y:  20, z:  16},
    Point{x: 300, y: -60, z:  16},
    Point{x: 280, y: -60, z:  16},
    Point{x: 280, y:  20, z: -16},
    Point{x: 300, y:  20, z: -16},
    Point{x: 300, y: -60, z: -16},
    Point{x: 280, y: -60, z: -16},

    Point{x: 320, y:  20, z:  16},
    Point{x: 340, y:  20, z:  16},
    Point{x: 340, y: -60, z:  16},
    Point{x: 320, y: -60, z:  16},
    Point{x: 320, y:  20, z: -16},
    Point{x: 340, y:  20, z: -16},
    Point{x: 340, y: -60, z: -16},
    Point{x: 320, y: -60, z: -16},

    Point{x: 300, y: -10, z:  16},
    Point{x: 320, y: -10, z:  16},
    Point{x: 320, y: -30, z:  16},
    Point{x: 300, y: -30, z:  16},
    Point{x: 300, y: -10, z: -16},
    Point{x: 320, y: -10, z: -16},
    Point{x: 320, y: -30, z: -16},
    Point{x: 300, y: -30, z: -16},

    Point{x: 295, y:  40, z:  16},
    Point{x: 325, y:  40, z:  16},
    Point{x: 325, y:  20, z:  16},
    Point{x: 295, y:  20, z:  16},
    Point{x: 295, y:  40, z: -16},
    Point{x: 325, y:  40, z: -16},
    Point{x: 325, y:  20, z: -16},
    Point{x: 295, y:  20, z: -16},
];

pub const EDGES: [(usize, usize); 264] = [
    // Rectangle 1 edges
    (0,1),(1,2),(2,3),(3,0),
    (4,5),(5,6),(6,7),(7,4),
    (0,4),(1,5),(2,6),(3,7),
    
    // Rectangle 2 edges
    (8,9),(9,10),(10,11),(11,8),
    (12,13),(13,14),(14,15),(15,12),
    (8,12),(9,13),(10,14),(11,15),
    
    // Rectangle 3 edges
    (16,17),(17,18),(18,19),(19,16),
    (20,21),(21,22),(22,23),(23,20),
    (16,20),(17,21),(18,22),(19,23),
    
    // Rectangle 4 edges
    (24,25),(25,26),(26,27),(27,24),
    (28,29),(29,30),(30,31),(31,28),
    (24,28),(25,29),(26,30),(27,31),
    
    // Rectangle 5 edges
    (32,33),(33,34),(34,35),(35,32),
    (36,37),(37,38),(38,39),(39,36),
    (32,36),(33,37),(34,38),(35,39),

    // Rectangle 1 edges (Left vertical bar)
    (40,41),(41,42),(42,43),(43,40),
    (44,45),(45,46),(46,47),(47,44),
    (40,44),(41,45),(42,46),(43,47),
    
    // Rectangle 2 edges (Top horizontal bar)
    (48,49),(49,50),(50,51),(51,48),
    (52,53),(53,54),(54,55),(55,52),
    (48,52),(49,53),(50,54),(51,55),
    
    // Rectangle 3 edges (Right vertical bar)
    (56,57),(57,58),(58,59),(59,56),
    (60,61),(61,62),(62,63),(63,60),
    (56,60),(57,61),(58,62),(59,63),
    
    // Rectangle 4 edges (Bottom horizontal bar)
    (64,65),(65,66),(66,67),(67,64),
    (68,69),(69,70),(70,71),(71,68),
    (64,68),(65,69),(66,70),(67,71),

    // Rectangle 1 edges (Left leg)
    (72,73),(73,74),(74,75),(75,72),
    (76,77),(77,78),(78,79),(79,76),
    (72,76),(73,77),(74,78),(75,79),
    
    // Rectangle 2 edges (Right leg)
    (80,81),(81,82),(82,83),(83,80),
    (84,85),(85,86),(86,87),(87,84),
    (80,84),(81,85),(82,86),(83,87),
    
    // Rectangle 3 edges (Left inner slope)
    (88,89),(89,90),(90,91),(91,88),
    (92,93),(93,94),(94,95),(95,92),
    (88,92),(89,93),(90,94),(91,95),
    
    // Rectangle 4 edges (Right inner slope)
    (96,97),(97,98),(98,99),(99,96),
    (100,101),(101,102),(102,103),(103,100),
    (96,100),(97,101),(98,102),(99,103),
    
    // Rectangle 5 edges (Bottom center)
    (104,105),(105,106),(106,107),(107,104),
    (108,109),(109,110),(110,111),(111,108),
    (104,108),(105,109),(106,110),(107,111),

    (112,113),(113,114),(114,115),(115,112),
    (116,117),(117,118),(118,119),(119,116),
    (112,116),(113,117),(114,118),(115,119),
    
    // Rectangle 2 edges (Diagonal slope)
    (120,121),(121,122),(122,123),(123,120),
    (124,125),(125,126),(126,127),(127,124),
    (120,124),(121,125),(122,126),(123,127),
    
    // Rectangle 3 edges (Right leg)
    (128,129),(129,130),(130,131),(131,128),
    (132,133),(133,134),(134,135),(135,132),
    (128,132),(129,133),(130,134),(131,135),

    // Letter "I" edges (single rectangle)
    (136,137),(137,138),(138,139),(139,136),
    (140,141),(141,142),(142,143),(143,140),
    (136,140),(137,141),(138,142),(139,143),
    
    // Letter "A" edges
    // Rectangle 1: Left diagonal slope
    (144,145),(145,146),(146,147),(147,144),
    (148,149),(149,150),(150,151),(151,148),
    (144,148),(145,149),(146,150),(147,151),
    
    // Rectangle 2: Right diagonal slope
    (152,153),(153,154),(154,155),(155,152),
    (156,157),(157,158),(158,159),(159,156),
    (152,156),(153,157),(154,158),(155,159),
    
    // Rectangle 3: Horizontal crossbar
    (160,161),(161,162),(162,163),(163,160),
    (164,165),(165,166),(166,167),(167,164),
    (160,164),(161,165),(162,166),(163,167),
    
    // Rectangle 4: Top center connector
    (168,169),(169,170),(170,171),(171,168),
    (172,173),(173,174),(174,175),(175,172),
    (168,172),(169,173),(170,174),(171,175),
];

// pub const VS: [Point; 8] = [
//     Point{x:  256, y:  256, z:  256},
//     Point{x: -256, y:  256, z:  256},
//     Point{x: -256, y: -256, z:  256},
//     Point{x:  256, y: -256, z:  256},
//     Point{x:  256, y:  256, z: -256},
//     Point{x: -256, y:  256, z: -256},
//     Point{x: -256, y: -256, z: -256},
//     Point{x:  256, y: -256, z: -256},
// ];
// 
// pub const EDGES: [(usize, usize); 12] = [
//     (0,1),(1,2),(2,3),(3,0),
//     (4,5),(5,6),(6,7),(7,4),
//     (0,4),(1,5),(2,6),(3,7),
// ];

pub fn rotate_xz(p: Point, angle: f32) -> Point {
    let c = (libm::cosf(angle) * FP as f32) as isize;
    let s = (libm::sinf(angle) * FP as f32) as isize;

    Point {
        x: (p.x * c - p.z * s) / FP,
        y: p.y,
        z: (p.x * s + p.z * c) / FP,
    }
}

pub fn translate_z(p: Point, dz: isize) -> Point {
    Point { x: p.x, y: p.y, z: p.z + dz }
}

pub fn project(p: Point) -> Point {
    Point {
        x: (p.x * FP) / p.z,
        y: (p.y * FP) / p.z,
        z: p.z,
    }
}

pub fn screen(p: Point) -> Point {
    Point {
        x: 512 + (p.x * SCALE) / FP,
        y: 384 - (p.y * SCALE) / FP,
        z: p.z,
    }
}

#[allow(static_mut_refs)]
pub unsafe fn frame(angle: &mut f32) {
    *angle += 0.08;

    let dz = 1024;

    unsafe {
        let fb = FRAMEBUFFER.as_mut().unwrap();
        fb.blit_rect_from_wallpaper((fb.width/4) as isize, (fb.height/4-30) as isize, fb.width/3+60, fb.height/3+60);
        //fb.blit_rect_from_wallpaper(1_isize, 1_isize, fb.width-2, fb.height-2)
    }

    for &(i, j) in &EDGES {
        let a = rotate_xz(VS[i], *angle);
        let b = rotate_xz(VS[j], *angle);

        let a = translate_z(a, dz);
        let b = translate_z(b, dz);

        if a.z <= 0 || b.z <= 0 {
            continue;
        }

        let a = screen(project(a));
        let b = screen(project(b));

        unsafe {
            let fb = FRAMEBUFFER.as_mut().unwrap();
            fb.draw_line(a.x, a.y, b.x, b.y, GREEN);
        }
    }

    unsafe {
        FRAMEBUFFER.as_mut().unwrap().draw_frame();
    }
}

#[allow(static_mut_refs)]
pub fn mk_bg() {
	let fb = unsafe{ FRAMEBUFFER.as_mut().unwrap() };
	fb.fill_screen(BLACK);

	unsafe {
	    let src = fb.double_buf.as_ptr();
	    let dst = fb.wallpaper_buf.as_mut_ptr();
	    core::ptr::copy_nonoverlapping(src, dst, fb.pitch * fb.height);
	}
}

const fn p(x: isize, y: isize, z: isize) -> Point {
    Point { x, y, z }
}

pub fn run() {
	mk_bg();
    let mut angle = 0.0;
    loop {
    	unsafe {
        	frame(&mut angle);
        }
        crate::time::sleep(800);
    }
}
