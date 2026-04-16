//! # Interrupt Descriptor Table (IDT)

use {
    crate::{
        apic, gdt,
        loader::global_loader,
        memory::kernel_address_space,
        scheduler::{self, with_scheduler},
    },
    log::{error, info},
    memory_types::VirtualAddress,
    x86_64::{
        registers::control::{Cr2, Cr3},
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
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3)
            .set_stack_index(gdt::USER_IST);
        IDT[scheduler::EXIT_INTERRUPT_NUMBER]
            .set_handler_fn(scheduler::exit_interrupt_handler)
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3)
            .set_stack_index(gdt::USER_IST);

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
    let addr_space_frame = Cr3::read_raw().0;
    let ins_ptr = stack_frame.instruction_pointer.as_ptr::<u8>();
    let opcode = unsafe { ins_ptr.read() };

    panic!("#DF({error_code}) at `{opcode:x}` in {addr_space_frame:?} : {stack_frame:#?}");
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let addr = Cr2::read_raw() as usize;
    let addr_space_frame = Cr3::read_raw().0;

    if error_code.contains(PageFaultErrorCode::USER_MODE) {
        if let Some(section) = global_loader()
            .get_section_for_addr(VirtualAddress::new(addr))
            .and_then(|weak| weak.upgrade())
        {
            with_scheduler(|scheduler| {
                let address_space = scheduler
                    .current_address_space()
                    .expect("should have an address space during user page fault");

                info!(
                    "Adding `{}` to `{}` at {:x}",
                    section.name,
                    address_space.name().expect("address space should be named"),
                    section.addr,
                );

                let mapping = section.mapping.lock();
                _ = mapping.map_into(&address_space, mapping.pages, mapping.flags);
            });
        } else {
            error!("#PF({error_code:?}) at {addr:#x} in {addr_space_frame:?}");
            scheduler::exit();
        }
    } else {
        panic!("#PF({error_code:?}) at {addr:#x} in {addr_space_frame:?} : {stack_frame:#?}",);
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let addr_space_frame = Cr3::read_raw().0;
    let ins_ptr = stack_frame.instruction_pointer.as_ptr::<u8>();
    let opcode = unsafe { ins_ptr.read() };

    if IO_PORT_OPCODES.contains(&opcode) {
        assert!(
            !kernel_address_space().is_current(),
            "Somehow, kernel failed an I/O operation! This shouldn't be possible!",
        );
        error!(
            "Attempted to use an I/O port without permission at `{opcode:x}` in \
            {addr_space_frame:?}",
        );
        scheduler::exit();
    } else {
        panic!(
            "#GP at `{opcode:x}` in {addr_space_frame:?}{} : {stack_frame:#?}",
            if error_code != 0 {
                format!(" for SEGMENT {error_code}")
            } else {
                format!("")
            },
        );
    }
}

const IO_PORT_OPCODES: &[u8] = &[0xE4, 0xE5, 0xE6, 0xE7, 0xEC, 0xED, 0xEE, 0xEF];
