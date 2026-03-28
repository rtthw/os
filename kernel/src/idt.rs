//! # Interrupt Descriptor Table (IDT)

use {
    crate::{apic, gdt, scheduler},
    log::info,
    x86_64::{
        registers::control::Cr2,
        set_general_handler,
        structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
    },
};



static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    info!("Initializing IDT...");

    unsafe {
        set_general_handler!(&mut IDT, unhandled_interrupt);

        IDT.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST);
        IDT.page_fault
            .set_handler_fn(page_fault_handler)
            .set_stack_index(gdt::PAGE_FAULT_IST);
        IDT.general_protection_fault
            .set_handler_fn(general_protection_fault_handler)
            .set_stack_index(gdt::GENERAL_PROTECTION_FAULT_IST);

        IDT[apic::TIMER_INDEX]
            .set_handler_fn(apic::timer_interrupt_handler)
            .set_stack_index(gdt::LOCAL_APIC_TIMER_IST);

        IDT[scheduler::DEFER_INTERRUPT_NUMBER]
            .set_handler_fn(scheduler::defer_interrupt_handler)
            .set_stack_index(gdt::DEFER_IST);

        IDT.load();
    }
}

fn unhandled_interrupt(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    panic!("UNHANDLED INTERRUPT: {index} ({error_code:?}) : {stack_frame:#?}");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!("DOUBLE_FAULT({error_code}) : {stack_frame:#?}");
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let addr = Cr2::read_raw();
    panic!("PAGE_FAULT({error_code:?}) @ {addr:#x} : {stack_frame:#?}");
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("GENERAL_PROTECTION_FAULT({error_code}) : {stack_frame:#?}");
}
