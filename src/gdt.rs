use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const GENERAL_PROTECTION_FAULT_IST_INDEX: u16 = 2;

lazy_static! {
	static ref GDT: (GlobalDescriptorTable, Selectors) = {
		let mut gdt = GlobalDescriptorTable::new();
		let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
		let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
		(gdt, Selectors { code_selector, tss_selector })
	};
}

struct Selectors {
	code_selector: SegmentSelector,
	tss_selector: SegmentSelector,
}

// lazy_static! {
// 	static ref GDT: (GlobalDescriptorTable, Selectors) = {
// 		let mut gdt = GlobalDescriptorTable::new();
// 		let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
// 
// 		//let tss_ptr = VirtAddr::from_ptr(unsafe { &RAW_TSS as *const _ });
// 		//let tss_size = core::mem::size_of::<RawTss>() as u32;
// 		let tss_selector = gdt.add_entry(unsafe { Descriptor::tss_segment(&TSS) });
// 
// 		(gdt, Selectors { code_selector, tss_selector })
// 	};
// }
// 
// struct Selectors {
// 	code_selector: SegmentSelector,
// 	tss_selector: SegmentSelector,
// }
// 
pub fn init() {
	use x86_64::instructions::tables::load_tss;
	use x86_64::instructions::segmentation::{CS, Segment};

	GDT.0.load();
	unsafe {
		CS::set_reg(GDT.1.code_selector);
		load_tss(GDT.1.tss_selector);
	}
}
// 
// 
// const IO_BITMAP_SIZE: usize = 8192 + 1;
// 
// #[repr(C, align(16))]
// struct RawTss {
//     tss: TaskStateSegment,
//     io_bitmap: [u8; IO_BITMAP_SIZE],
// }
// 
// static mut RAW_TSS: RawTss = RawTss {
//     tss: TaskStateSegment::new(),
//     io_bitmap: [0; IO_BITMAP_SIZE],
// };
// 
// lazy_static! {
//     pub static ref TSS: &'static TaskStateSegment = {
//         unsafe {
//             let raw_tss_ptr = &raw mut RAW_TSS as *mut RawTss;
// 
//             // STACK
//             const STACK_SIZE: usize = 4096 * 5;
//             static mut DF_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
// 
//             let df_stack_ptr = &raw mut DF_STACK as *mut u8;
//             let stack_end = VirtAddr::from_ptr(df_stack_ptr.add(STACK_SIZE));
// 
//             (*raw_tss_ptr).tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
// 
//             // iomap_base указывает за пределы TSS → доступ ко всем портам
//             (*raw_tss_ptr).tss.iomap_base = 0xFFFFu16;
// 
//             // Возвращаем ссылку на безопасную часть — TSS
//             &(*raw_tss_ptr).tss
//         }
//     };
// }

lazy_static! {
	static ref TSS: TaskStateSegment = {
		let mut tss = TaskStateSegment::new();
		tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
			const STACK_SIZE: usize = 4096 * 5;
			static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

			let stack_start = VirtAddr::from_ptr(&raw const STACK);
			let stack_end = stack_start + STACK_SIZE;
			stack_end
		};

		tss.interrupt_stack_table[GENERAL_PROTECTION_FAULT_IST_INDEX as usize] = {
			const STACK_SIZE: usize = 4096 * 5;
			static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

			let stack_start = VirtAddr::from_ptr(&raw const STACK);
			let stack_end = stack_start + STACK_SIZE;
			stack_end
		};

		tss.iomap_base = 0xFFFFu16;
		
		tss
	};
}
// 
// pub fn get_tss_ptr() -> *const TaskStateSegment {
//     &*TSS as *const _
// }
