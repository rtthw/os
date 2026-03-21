//! # Global Descriptor Table (GDT)

use {
    core::ptr::addr_of,
    log::info,
    memory_types::PAGE_SIZE,
    x86_64::{
        VirtAddr,
        instructions::tables::load_tss,
        registers::segmentation::{CS, DS, SS, Segment},
        structures::{
            gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
            tss::TaskStateSegment,
        },
    },
};



static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut SELECTORS: Selectors = Selectors::NULL;

const INTERRUPT_STACK_SIZE: usize = PAGE_SIZE * 8;

pub const DOUBLE_FAULT_IST: u16 = 0;
pub const PAGE_FAULT_IST: u16 = 1;
pub const GENERAL_PROTECTION_FAULT_IST: u16 = 2;
pub const LOCAL_APIC_TIMER_IST: u16 = 3;

pub fn init() {
    info!("Initializing GDT...");

    unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST as usize] = {
            static mut STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + INTERRUPT_STACK_SIZE as u64
        };
        TSS.interrupt_stack_table[PAGE_FAULT_IST as usize] = {
            static mut STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + INTERRUPT_STACK_SIZE as u64
        };
        TSS.interrupt_stack_table[GENERAL_PROTECTION_FAULT_IST as usize] = {
            static mut STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + INTERRUPT_STACK_SIZE as u64
        };
        TSS.interrupt_stack_table[LOCAL_APIC_TIMER_IST as usize] = {
            static mut STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + INTERRUPT_STACK_SIZE as u64
        };

        let kernel_tss = GDT.append(Descriptor::tss_segment(&TSS));
        let kernel_code = GDT.append(Descriptor::kernel_code_segment());
        let kernel_data = GDT.append(Descriptor::kernel_data_segment());
        let user_code = GDT.append(Descriptor::user_code_segment());
        let user_data = GDT.append(Descriptor::user_data_segment());

        SELECTORS = Selectors {
            kernel_tss,
            kernel_code,
            kernel_data,
            user_code,
            user_data,
        };

        // debug!("SELECTORS:\n{:#?}", SELECTORS);

        GDT.load();

        CS::set_reg(SELECTORS.kernel_code);
        DS::set_reg(SELECTORS.kernel_data);

        // Without this, you get a general protection fault during the end-of-interrupt
        // signal of the local APIC timer.
        SS::set_reg(SegmentSelector(0));

        load_tss(SELECTORS.kernel_tss);
    }
}

#[derive(Debug)]
pub struct Selectors {
    kernel_tss: SegmentSelector,
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub user_data: SegmentSelector,
}

impl Selectors {
    const NULL: Self = Self {
        kernel_tss: SegmentSelector::NULL,
        kernel_code: SegmentSelector::NULL,
        kernel_data: SegmentSelector::NULL,
        user_code: SegmentSelector::NULL,
        user_data: SegmentSelector::NULL,
    };
}
