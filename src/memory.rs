use multiboot2::{ MemoryArea, BootInformation, ElfSection, ElfSectionFlags };
use core::ops::{ Index, IndexMut, Deref, DerefMut };
use core::marker::PhantomData;
use core::ptr::Unique;
use crate::println;

pub const PAGE_SIZE: usize = 4096;
pub const ENTRY_COUNT: usize = 512;
pub const P4: *mut Table<Level4> = 0xffffffff_fffff000 as *mut _; 

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

#[derive(Debug, Clone, Copy)]
pub struct Page {
	number: usize,
}

impl Page {
	pub fn containing_address(address: VirtualAddress) -> Page {
		assert!(address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000, "invalid address 0x{:x}", address);
		Page { number: address / PAGE_SIZE }
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
		use x86_64::PhysAddr;
		use x86_64::registers::control;
		use x86_64::structures::paging::PhysFrame;

		let (_, cr3_address) = control::Cr3::read_raw();
		let old_table = InactivePageTable {
			p4_frame: Frame::containing_address(cr3_address as usize),
		};

		unsafe {
			ActivePageTable::write_raw_cr3(new_table.p4_frame.start_address() as u64);
		}

		old_table
	}

	#[inline]
	unsafe fn write_raw_cr3(address: u64) {
		use core::arch::asm;
		unsafe {
			asm!("mov cr3, {}", in(reg) address, options(nostack, nomem, preserves_flags));
		}
	}
}

pub fn remap_kernel<A>(allocator: &mut A, boot_info: &BootInformation) where A: FrameAllocator {
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
			    //println!("skipping non-page-aligned section at addr: {:#x}, size: {:#x}", section.start_address(), section.size());
			    continue;
			}
			
			//println!("mapping section at addr: {:#x}, size: {:#x}", section.start_address(), section.size());
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
	println!("NEW TABLE!");

	let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
	active_table.unmap(old_p4_page, allocator);
	println!("guard page at {:#x}", old_p4_page.start_address());
}

