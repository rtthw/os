//! # Interrupt Descriptor Table (IDT)

use {
    crate::gdt,
    log::info,
    x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};



static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    info!("Initializing GDT...");

    #[allow(static_mut_refs)]
    unsafe {
        IDT.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST);

        IDT.load();
    }
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("DOUBLE_FAULT : {:#?}", stack_frame);
}
