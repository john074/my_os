use multiboot2::{ MemoryArea, BootInformation, ElfSection,  BootInformationHeader };
use core::ops::{ Index, IndexMut, Deref, DerefMut };
use core::marker::PhantomData;
use core::ptr::{ Unique, self,  null_mut };
use core::cell::UnsafeCell;
use core::mem;
use alloc::alloc::{ GlobalAlloc, Layout };
use crate::cpu::{ cr3, write_raw_cr3 }; 
use crate::println;

pub const PAGE_SIZE: usize = 4096;
pub const ENTRY_COUNT: usize = 512;
pub const P4: *mut Table<Level4> = 0xffffffff_fffff000 as *mut _; 
pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024;
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

bitflags! {
	pub struct EntryFlags: u64 {
		const PRESENT = 		1 << 0;
		const WRITABLE = 		1 << 1;
		const USER_ACCESSIBLE = 1 << 2;
		const WRITE_THROUGH = 	1 << 3;
		const NO_CACHE = 		1 << 4;
		const ACCESSED = 		1 << 5;
		const DIRTY =			1 << 6;
		const HUGE_PAGE = 		1 << 7;
		const GLOBAL = 			1 << 8;
		const NO_EXECUTE = 		1 << 63;
	}
}

impl EntryFlags {
	pub fn from_elf_section_flags(section: &ElfSection) -> EntryFlags {
		let mut flags = EntryFlags::empty();

		let mut is_executable: bool = false;
		let mut is_present: bool = false;
		let mut is_writable: bool = false;
		
		for f in section.flags() {
			if f.bits() == 0x1 {
				is_present = true;	
			}

			if f.bits() == 0x2 {
				is_writable = true;	
			}

			if f.bits() == 0x4 {
				is_executable = true;	
			}
		}

		if is_present {
			flags = flags | PRESENT;
		}
		if is_writable {
			flags = flags | WRITABLE;
		}
		if !is_executable {
			flags = flags | NO_EXECUTE;
		}
		
		flags
	}
}

pub fn init(multiboot_information_address: usize) {
	let boot_info = unsafe{ BootInformation::load(multiboot_information_address as *const BootInformationHeader).unwrap() };
	
	let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");
	 	
	let elf_sections_tag = boot_info.elf_sections().expect("Elf-sections tag required");
	let kernel_start = elf_sections_tag.map(|s| s.start_address()).min().unwrap();
	
	let elf_sections_tag = boot_info.elf_sections().expect("Elf-sections tag required");
	let kernel_end = elf_sections_tag.map(|s| s.start_address()).max().unwrap();
	println!("Kernel start: {:#x}, kernel end: {:#x}", kernel_start, kernel_end);
		
	let multiboot_start = multiboot_information_address;
	let multiboot_end = multiboot_start + (boot_info.total_size());
	
	let mut frame_allocator = AreaFrameAllocator::new(kernel_start as usize, kernel_end as usize, multiboot_start, multiboot_end, memory_map_tag.memory_areas());
	let mut active_table = remap_kernel(&mut frame_allocator, &boot_info);

	let heap_start_page = Page::containing_address(HEAP_START);
	let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

	for page in Page::range_inclusive(heap_start_page, heap_end_page) {
		active_table.map(page, WRITABLE, &mut frame_allocator);
	}

	unsafe {
		ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
	}
	println!("Memory initialization\t[OK]");
}


//******* Frame Allocator *******\\

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
	number: usize,
}

impl Frame {
	fn containing_address(address: usize) -> Frame {
		Frame { number: address / PAGE_SIZE }
	}

	fn start_address(&self) -> PhysicalAddress {
		self.number * PAGE_SIZE
	}

	fn clone(&self) -> Frame {
		Frame { number: self.number }
	}

	fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
		FrameIter {
			start: start,
			end: end,
		}
	}
}

struct FrameIter {
	start: Frame,
	end: Frame,
}

impl Iterator for FrameIter {
	type Item = Frame;

	fn next(&mut self) -> Option<Frame> {
		if self.start <= self.end {
			let frame = self.start.clone();
			self.start.number += 1;
			Some(frame)
		} else {
			None
		}
	}
}

pub trait FrameAllocator {
	fn allocate_frame(&mut self) -> Option<Frame>;
	fn deallocate_frame(&mut self, frame: Frame);
}

pub struct AreaFrameAllocator<'a> {
	next_free_frame: Frame,
	current_area: Option<&'a MemoryArea>,
	areas: &'a [MemoryArea],
	kernel_start: Frame,
	kernel_end: Frame,
	multiboot_start: Frame,
	multiboot_end: Frame,
}

impl<'a> FrameAllocator for AreaFrameAllocator<'a> {
	fn allocate_frame(&mut self) -> Option<Frame> {
		if let Some(area) = self.current_area {
			let frame = Frame { number: self.next_free_frame.number };
			
			let current_area_last_frame = {
				let address = area.start_address() + area.size() - 1;
				Frame::containing_address(address as usize)	
			};

			if frame > current_area_last_frame {
				self.choose_next_area();
			} else if frame >= self.kernel_start && frame <= self.kernel_end {
				self.next_free_frame = Frame {
					number: self.kernel_end.number + 1	
				};
			} else if frame >= self.multiboot_start && frame <= self.multiboot_end {
				self.next_free_frame = Frame {
					number: self.multiboot_end.number + 1
				};
			} else {
				self.next_free_frame.number += 1;
				//println!("3 - frame allocated at address {:#?}", frame.start_address());
				return Some(frame);
			}

			self.allocate_frame()
		} else {
			None
		}
	}

	fn deallocate_frame(&mut self, frame: Frame) {
		unimplemented!()
	}
}

impl<'a> AreaFrameAllocator<'a> {
	pub fn new(kernel_start: usize, kernel_end: usize, multiboot_start: usize, multiboot_end: usize, memory_areas: &'a[MemoryArea]) -> AreaFrameAllocator<'a> {
		let mut allocator = AreaFrameAllocator {
			next_free_frame: Frame::containing_address(0),
			current_area: None,
			areas: memory_areas,
			kernel_start: Frame::containing_address(kernel_start),
			kernel_end: Frame::containing_address(kernel_end),
			multiboot_start: Frame::containing_address(multiboot_start),
			multiboot_end: Frame::containing_address(multiboot_end),
		};
		allocator.choose_next_area();
		allocator
	}

	fn choose_next_area(&mut self) {
		self.current_area = self.areas.iter().filter(|area| {
			let address = area.start_address() + area.size() - 1;
			Frame::containing_address(address as usize) >= self.next_free_frame
		}).min_by_key(|area| area.start_address());

		if let Some(area) = self.current_area {
			let start_frame = Frame::containing_address(area.start_address() as usize);
			if self.next_free_frame < start_frame {
				self.next_free_frame = start_frame;
			}
		}
	}
}

//******* PAGING *******\\

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
	number: usize,
}

impl Page {
	pub fn containing_address(address: VirtualAddress) -> Page {
		assert!(address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000, "invalid address 0x{:x}", address);
		Page { number: address / PAGE_SIZE }
	}

	pub fn range_inclusive(start: Page, end: Page) -> PageIter {
		PageIter {
			start: start,
			end: end
		}
	}

	fn start_address(&self) -> usize {
		self.number * PAGE_SIZE
	}

	fn p4_index(&self) -> usize {
		(self.number >> 27) & 0o777
	}

	fn p3_index(&self) -> usize {
		(self.number >> 18) & 0o777
	}

	fn p2_index(&self) -> usize {
		(self.number >> 9) & 0o777
	}

	fn p1_index(&self) -> usize {
		(self.number) & 0o777
	}
}

pub struct PageIter {
	start: Page,
	end: Page,
}

impl Iterator for PageIter {
	type Item = Page;

	fn next(&mut self) -> Option<Page> {
		if self.start <= self.end {
			let page = self.start;
			self.start.number += 1;
			Some(page)
		} else {
			None
		}
	}
}

pub struct Entry(u64);

impl Entry {
	pub fn is_unused(&self) -> bool {
		self.0 == 0
	}

	pub fn set_unused(&mut self) {
		self.0 = 0;
	}

	pub fn flags(&self) -> EntryFlags {
		EntryFlags::from_bits_truncate(self.0)
	}
	
	pub fn pointed_frame(&self) -> Option<Frame> {
		if self.flags().contains(PRESENT) {
			Some(Frame::containing_address(self.0 as usize & 0x000fffff_fffff000))
		}
		else {
			None
		}
	}
	
	pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
		assert!(frame.start_address() & !0x000fffff_fffff000 == 0);
		self.0 = (frame.start_address() as u64) | flags.bits();
	}
}

pub struct Table<L: TableLevel> {
	entries: [Entry; ENTRY_COUNT],
	level: PhantomData<L>,
}

impl<L> Table<L> where L: TableLevel {
	pub fn zero(&mut self) {
		for entry in self.entries.iter_mut() {
			entry.set_unused();
		}
	}
}

impl<L> Index<usize> for Table<L> where L: TableLevel {
	type Output = Entry;

	fn index(&self, index: usize) -> &Entry {
		&self.entries[index]
	}
}

impl<L> IndexMut<usize> for Table<L> where L: TableLevel {
	fn index_mut(&mut self, index: usize) -> &mut Entry {
		&mut self.entries[index]
	}
}

impl<L> Table<L> where L: HeirarchicalLevel {
	fn next_table_address(&self, index: usize) -> Option<usize> {
		let entry_flags = self[index].flags();
		if entry_flags.contains(PRESENT) && !entry_flags.contains(HUGE_PAGE) {
			let table_address = self as *const _ as usize;
			Some((table_address << 9) | (index << 12))
		} else {
			None
		}
	}

	pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
		self.next_table_address(index).map(|address| unsafe { &*(address as *const _) })
	}

	pub fn next_table_mut(&mut self, index: usize) -> Option<&mut Table<L::NextLevel>> {
		self.next_table_address(index).map(|address| unsafe { &mut *(address as *mut _) })
	}

	pub fn next_table_create<A>(&mut self, index: usize, allocator: &mut A) -> &mut Table<L::NextLevel> where A: FrameAllocator {
		if self.next_table(index).is_none() {
			assert!(!self.entries[index].flags().contains(HUGE_PAGE), "huge pages are not supported");
			let frame = allocator.allocate_frame().expect("no frames available");
			self.entries[index].set(frame, PRESENT | WRITABLE);
			self.next_table_mut(index).unwrap().zero();
		}		
		self.next_table_mut(index).unwrap()
	}
}

pub struct Mapper {
	p4: Unique<Table<Level4>>,
}

impl Mapper {
	pub unsafe fn new() -> Mapper {
		Mapper {
			p4: unsafe{ Unique::new_unchecked(P4) }
		}
	}

	pub fn p4(&self) -> &Table<Level4> {
		unsafe { self.p4.as_ref() }
	}

	pub fn p4_mut(&mut self) -> &mut Table<Level4> {
		unsafe { self.p4.as_mut() }
	}

	pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
		let offset = virtual_address % PAGE_SIZE;
		self.translate_page(Page::containing_address(virtual_address)).map(|frame| frame.number * PAGE_SIZE + offset)
	}

	pub fn translate_page(&self, page: Page) -> Option<Frame> {
		let p3 = self.p4().next_table(page.p4_index());
		let huge_page = || {
			p3.and_then(|p3| {
				let p3_entry = &p3[page.p3_index()];

				if let Some(start_frame) = p3_entry.pointed_frame() {
					if p3_entry.flags().contains(HUGE_PAGE) {
						assert!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0);
						return Some(Frame{ number: start_frame.number + page.p2_index() * ENTRY_COUNT + page.p1_index() });
					}
				}
				
				if let Some(p2) = p3.next_table(page.p3_index()) {
					let p2_entry = &p2[page.p2_index()];

					if let Some(start_frame) = p2_entry.pointed_frame() {
						if p2_entry.flags().contains(HUGE_PAGE) {
							assert!(start_frame.number % ENTRY_COUNT == 0);
							return Some(Frame { number: start_frame.number + page.p1_index() });
						}
					}
				} 
				None
			})
		};

		p3.and_then(|p3| p3.next_table(page.p3_index())).and_then(|p2| p2.next_table(page.p2_index())).and_then(|p1| p1[page.p1_index()].pointed_frame()).or_else(huge_page)
	}

	pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
		let mut p4 = self.p4_mut();
		let mut p3 = p4.next_table_create(page.p4_index(), allocator);
		let mut p2 = p3.next_table_create(page.p3_index(), allocator);
		let mut p1 = p2.next_table_create(page.p2_index(), allocator);

		assert!(p1[page.p1_index()].is_unused());
		p1[page.p1_index()].set(frame, flags | PRESENT);
	}

	pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
		let frame = allocator.allocate_frame().expect("out of memory");
		self.map_to(page, frame, flags, allocator)
	}

	pub fn identity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
		let page = Page::containing_address(frame.start_address());
		self.map_to(page, frame, flags, allocator)
	}

	pub fn unmap<A>(&mut self, page: Page, allocator: &mut A) where A: FrameAllocator {
		use x86_64::instructions::tlb;
		use x86_64::addr::VirtAddr;
		//println!("9 - address: {:#?}, mapped: {:#?}", page.start_address(), self.translate(page.start_address()).is_some());
		//assert!(self.translate(page.start_address()).is_some());
		if self.translate(page.start_address()).is_none() {
			return;
		}
		let p1 = self.p4_mut().next_table_mut(page.p4_index()).and_then(|p3| p3.next_table_mut(page.p3_index())).and_then(|p2| p2.next_table_mut(page.p2_index())).expect("no huge pages");
		let frame = p1[page.p1_index()].pointed_frame().unwrap();
		p1[page.p1_index()].set_unused();
		tlb::flush(VirtAddr::new(page.start_address() as u64));		
		//todo: free empty p1-p3 (allocator.dealloc_frame(frame))
	}
	
}

pub trait TableLevel {}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

pub trait HeirarchicalLevel: TableLevel {
	type NextLevel: TableLevel;
}

impl HeirarchicalLevel for Level4 {
	type NextLevel = Level3;
}

impl HeirarchicalLevel for Level3 {
	type NextLevel = Level2;
}

impl HeirarchicalLevel for Level2 {
	type NextLevel = Level1;
}

//****** REMAPING *******

pub struct InactivePageTable {
	p4_frame: Frame, 
}

impl InactivePageTable {
	pub fn new(frame: Frame, active_table: &mut ActivePageTable, temporary_page: &mut TemporaryPage) -> InactivePageTable {
		{
			//println!("4 - new inactive page table at frame {:#?}", frame.start_address());
			let table = temporary_page.map_table_frame(frame.clone(), active_table);
			table.zero();
			table[511].set(frame.clone(), PRESENT | WRITABLE);
		}
		//println!("7");
		temporary_page.unmap(active_table);
		InactivePageTable { p4_frame: frame }
	}
}

pub struct TemporaryPage {
	page: Page,
	allocator: TinyAllocator,
}

impl TemporaryPage {
	pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage where A: FrameAllocator {
		//println!("1");
		TemporaryPage { page, allocator: TinyAllocator::new(allocator) }
	}

	pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> VirtualAddress {
		//println!("5 - map in temp page");
		assert!(active_table.translate_page(self.page).is_none(), "temporary page is already mapped");
		active_table.map_to(self.page, frame, WRITABLE, &mut self.allocator);
		self.page.start_address()
	}

	pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
		//println!("8");
		active_table.unmap(self.page, &mut self.allocator)
	}

	pub fn map_table_frame(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> &mut Table<Level1> {
		//println!("4.5 - map table frame: {:#?}", frame.start_address());
		unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>)}
	}
}

struct TinyAllocator([Option<Frame>; 3]);

impl FrameAllocator for TinyAllocator {
	fn allocate_frame(&mut self) -> Option<Frame> {
		for frame_option in &mut self.0 {
			if frame_option.is_some() {
				return frame_option.take();
			}
		}
		None
	}

	fn deallocate_frame(&mut self, frame: Frame) {
		for frame_option in &mut self.0 {
			if frame_option.is_none() {
				*frame_option = Some(frame);
				return;
			}
		}
		panic!("Can hold only 3 frames");
	}
}

impl TinyAllocator {
	fn new<A>(allocator: &mut A) -> TinyAllocator where A: FrameAllocator {
		let mut f = || allocator.allocate_frame();
		let frames = [f(), f(), f()];
		TinyAllocator(frames)
	}
}

pub struct ActivePageTable {
	mapper: Mapper,
}

impl Deref for ActivePageTable {
	type Target = Mapper;
	fn deref(&self) -> &Mapper {
		&self.mapper
	}
}

impl DerefMut for ActivePageTable {
	fn deref_mut(&mut self) -> &mut Mapper {
		&mut self.mapper
	}
}

impl ActivePageTable {
	unsafe fn new() -> ActivePageTable {
		//println!("2 - new active page table");
		ActivePageTable { mapper: unsafe{ Mapper::new() } }
	}

	pub fn with<F>(&mut self, table: &mut InactivePageTable, temporary_page: &mut TemporaryPage, f:F) where F: FnOnce(&mut Mapper)
	{
		use x86_64::instructions::tlb;
		use x86_64::registers::control;
		{
			let (_, cr3_address) = control::Cr3::read_raw();
			let backup = Frame::containing_address(cr3_address as usize);

			let p4_table = temporary_page.map_table_frame(backup.clone(), self);
			
			self.p4_mut()[511].set(table.p4_frame.clone(), PRESENT | WRITABLE);
			tlb::flush_all();
			f(self);	
			p4_table[511].set(backup, PRESENT | WRITABLE);
			tlb::flush_all();
		}
		temporary_page.unmap(self);
	}

	pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
		let old_table = InactivePageTable {
			p4_frame: Frame::containing_address(cr3() as usize),
		};

		unsafe {
			write_raw_cr3(new_table.p4_frame.start_address() as u64);
		}

		old_table
	}
}

pub fn remap_kernel<A>(allocator: &mut A, boot_info: &BootInformation) -> ActivePageTable where A: FrameAllocator {
	println!("Remapping kernel.");
	let mut temporary_page = TemporaryPage::new(Page { number: 0x123456789 }, allocator);
	let mut active_table = unsafe { ActivePageTable::new() };
	let mut new_table = {
		let frame = allocator.allocate_frame().expect("no more frames");
		InactivePageTable::new(frame, &mut active_table, &mut temporary_page)	
	};

	active_table.with(&mut new_table, &mut temporary_page, |mapper| {
		let elf_sections_tag = boot_info.elf_sections().expect("Elf-sections tag required");

		for section in elf_sections_tag {
			if !section.is_allocated() {
				continue;
			}

			if section.start_address() as usize % PAGE_SIZE != 0 {
			    println!("Skipping non-page-aligned section at address: {:#x}, size: {:#x}", section.start_address(), section.size());
			    continue;
			}
			
			println!("Mapping section at address: {:#x}, size: {:#x}", section.start_address(), section.size());
			//assert!(section.start_address() as usize % PAGE_SIZE == 0, "sections need to be page aligned");

			let flags = EntryFlags::from_elf_section_flags(&section);

			let start_frame = Frame::containing_address(section.start_address() as usize);
			let end_frame = Frame::containing_address((section.end_address() - 1) as usize);
			for frame in Frame::range_inclusive(start_frame, end_frame) {
				mapper.identity_map(frame, flags, allocator);
			}
		}

		let vga_buffer_frame = Frame::containing_address(0xb8000);
		mapper.identity_map(vga_buffer_frame, WRITABLE, allocator);

		let multiboot_start = Frame::containing_address(boot_info.start_address());
		let multiboot_end = Frame::containing_address(boot_info.end_address() - 1);
		for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
			mapper.identity_map(frame, PRESENT, allocator);
		}
	});

	let old_table = active_table.switch(new_table);
	println!("Page table successfuly swaped.");

	let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
	active_table.unmap(old_p4_page, allocator);
	println!("Guard page at {:#x}.", old_p4_page.start_address());

	active_table
}

//Allocator

pub struct Locked<A> {
	inner: spin::Mutex<A>
}

impl<A> Locked<A> {
	pub const fn new(inner: A) -> Self {
		Locked {
			inner: spin::Mutex::new(inner)
		}
	}

	pub fn lock(&self) -> spin::MutexGuard<A> {
		self.inner.lock()
	}
}

struct FixedSizeListNode {
	next: Option<&'static mut FixedSizeListNode>
}

pub struct FixedSizeBlockAllocator {
	list_heads: [Option<&'static mut FixedSizeListNode>; BLOCK_SIZES.len()],
	//fallback_allocator: BumpAllocator
	fallback_allocator: Locked<LinkedListAllocator>
}

impl FixedSizeBlockAllocator {
	pub const fn new() -> Self {
		const EMPTY: Option<&'static mut FixedSizeListNode> = None;
		FixedSizeBlockAllocator {
			list_heads: [EMPTY; BLOCK_SIZES.len()],
			fallback_allocator: Locked::new(LinkedListAllocator::new())
		// 	fallback_allocator: BumpAllocator {
		// 		heap_start: 0x_4444_4444_0000,
		// 		heap_end: 0x_4444_4444_0000 + 100 * 1024,
		// 		next: UnsafeCell::new(0x_4444_4444_0000),	
		// 	}
		}
		
	}

	// pub unsafe fn init(&mut self) {
	// 	for (i, &block_size) in BLOCK_SIZES.iter().enumerate() {
	//     	let layout = Layout::from_size_align(block_size, block_size).unwrap();
	//     	let ptr = self.fallback_allocator.alloc(layout);
	//     	if !ptr.is_null() {
	//    			let node_ptr = ptr as *mut FixedSizeListNode;
	//         	node_ptr.write(FixedSizeListNode { next: None });
	//         	self.list_heads[i] = Some(&mut *node_ptr);
	//     	}
	// 	}
	// }

	pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
		unsafe {
			self.fallback_allocator.lock().init(heap_start, heap_size);
		}
	} 
}

fn list_index(layout: &Layout) -> Option<usize> {
	let required_block_size = layout.size().max(layout.align());
	BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let mut allocator = self.lock();
		match list_index(&layout) {
			Some(index) => {
				match allocator.list_heads[index].take() {
					Some(node) => {
						allocator.list_heads[index] = node.next.take();
						node as *mut FixedSizeListNode as *mut u8
					}
					None => {
						let block_size = BLOCK_SIZES[index];
						let block_align = block_size;
						let layout = Layout::from_size_align(block_size, block_align).unwrap();
						allocator.fallback_allocator.alloc(layout)
					}
				}
			}
			None => allocator.fallback_allocator.alloc(layout)
		}
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		let mut allocator = self.lock();
		match list_index(&layout) {
			Some(index) => {
				let new_node = FixedSizeListNode {
					next: allocator.list_heads[index].take()
				};
				assert!(mem::size_of::<FixedSizeListNode>() <= BLOCK_SIZES[index]);
				assert!(mem::align_of::<FixedSizeListNode>() <= BLOCK_SIZES[index]);
				let new_node_ptr = ptr as *mut FixedSizeListNode;
				unsafe {
					new_node_ptr.write(new_node);
					allocator.list_heads[index] = Some(&mut *new_node_ptr);
				}
			}
			None => {
				//let ptr = NonNull::new(ptr).unwrap();
				//unimplemented!()
				allocator.fallback_allocator.dealloc(ptr, layout);
			}
		}
	}
}

struct VarSizeListNode {
	size: usize,
	next: Option<&'static mut VarSizeListNode>
}

impl VarSizeListNode {
	const fn new(size: usize) -> Self {
		VarSizeListNode { size, next: None }
	}

	fn start_addr(&self) -> usize {
		self as *const Self as usize
	}

	fn end_addr(&self) -> usize {
		self.start_addr() + self.size
	}
}

pub struct LinkedListAllocator {
	head: VarSizeListNode,
}

impl LinkedListAllocator {
	pub const fn new() -> Self {
		Self {
			head: VarSizeListNode::new(0),
		}
	}

	pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
		unsafe {
			self.add_free_region(heap_start, heap_size);
		}
	}

	unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
		assert_eq!(align_up(addr, mem::align_of::<VarSizeListNode>()), addr);
		assert!(size >= mem::size_of::<VarSizeListNode>());

		let mut node = VarSizeListNode::new(size);
		node.next = self.head.next.take();
		let node_ptr = addr as *mut VarSizeListNode;
		unsafe {
			node_ptr.write(node);
			self.head.next = Some(&mut *node_ptr);
		}
	}

	fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut VarSizeListNode, usize)> {
		let mut current = &mut self.head;

		while let Some(ref mut region) = current.next {
			if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
				let next = region.next.take();
				let ret = Some((current.next.take().unwrap(), alloc_start));
				current.next = next;
				return ret;
			} else {
				current = current.next.as_mut().unwrap();
			}
		}
		None
	}

	fn alloc_from_region(region: &VarSizeListNode, size: usize, align: usize) -> Result<usize, ()> {
		let alloc_start = align_up(region.start_addr(), align);
		let alloc_end = alloc_start.checked_add(size).ok_or(())?;

		
		if alloc_end > region.end_addr() {
			return Err(());
		}
		
		let excess_size = region.end_addr() - alloc_end;
		if excess_size > 0 && excess_size < mem::size_of::<VarSizeListNode>() {
			return Err(());
		}

		Ok(alloc_start)
	}

	fn size_align(layout: Layout) -> (usize, usize) {
		let layout = layout.align_to(mem::align_of::<VarSizeListNode>()).expect("failed to adjust aligment").pad_to_align();
		let size = layout.size().max(mem::size_of::<VarSizeListNode>());
		(size, layout.align())
	}
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let (size, align) = LinkedListAllocator::size_align(layout);
		let mut allocator = self.lock();
		if let Some((region, alloc_start)) = allocator.find_region(size, align) {
			let alloc_end = alloc_start.checked_add(size).expect("overflow");
			let excess_size = region.end_addr() - alloc_end;
			if excess_size > 0 {
				unsafe {
					allocator.add_free_region(alloc_end, excess_size);
				}
			}
			alloc_start as *mut u8
		} else {
			ptr::null_mut()
		}
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		println!("...");
		let (size, _) = LinkedListAllocator::size_align(layout);
		unsafe {
			self.lock().add_free_region(ptr as usize, size)
		}
	}
}

struct BumpAllocator {
	heap_start: usize,
	heap_end: usize,
	next: UnsafeCell<usize>,
}

unsafe impl GlobalAlloc for BumpAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let current = *self.next.get();
		let alloc_start = align_up(current, layout.align());
		let alloc_end = alloc_start.saturating_add(layout.size());

		if alloc_end > self.heap_end {
			null_mut()
		}
		else {
			*self.next.get() = alloc_end;
			alloc_start as *mut u8
		}
	}

	unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
		// wasnt a part of the deal
	}
}

unsafe impl Sync for BumpAllocator {}

fn align_up(addr: usize, align: usize) -> usize {
	(addr + align - 1) & !(align - 1)
}
