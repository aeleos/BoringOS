//! Handles interrupts on the x86_64 architecture.

pub mod lapic;
mod ioapic;

pub use self::lapic::issue_self_interrupt;
use multitasking::scheduler::schedule_next_thread;
use x86_64::registers::control_regs;
use x86_64::structures::idt::{Idt, ExceptionStackFrame, PageFaultErrorCode};
use x86_64::instructions::interrupts;

/// The interrupt number for the scheduling interrupt.
pub const SCHEDULE_INTERRUPT_NUM: u8 = 0x22;

lazy_static! {
    /// The interrupt descriptor table used by the kernel.
    static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.divide_by_zero.set_handler_fn(divide_by_zero_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.interrupts[0].set_handler_fn(timer_handler);
        idt.interrupts[1].set_handler_fn(irq1_handler);
        idt.interrupts[2].set_handler_fn(schedule_interrupt).disable_interrupts(false);
        // Spurious interrupt handler.
        idt.interrupts[0xf].set_handler_fn(empty_handler);

        idt
    };
}

/// Initializes interrupts on the x86_64 architecture.
pub fn init() {
    assert_has_not_been_called!("Interrupts should only be initialized once.");

    IDT.load();

    lapic::init();
    lapic::set_periodic_timer(150);

    ioapic::init();
}

macro_rules! irq_interrupt {
    ($(#[$attr: meta])* fn $name: ident $content: tt) => {
        $(#[$attr])*
        extern "x86-interrupt" fn $name(_: &mut ExceptionStackFrame) {
            let old_priority = lapic::get_priority();
            lapic::set_priority(0x20);
            unsafe {
                interrupts::enable();
            }

            $content

            unsafe {
                interrupts::disable();
            }
            lapic::signal_eoi();
            lapic::set_priority(old_priority);
        }
    };
}

/// The divide by zero exception handler of the kernel.
extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("DIVIDE BY ZERO!");
    println!("{:?}", stack_frame);
    loop {}
}

/// The breakpoint exception handler of the kernel.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("BREAKPOINT");
    println!("{:?}", stack_frame);
    loop {}
}

/// The page fault handler of the kernel.
extern "x86-interrupt" fn page_fault_handler(stack_frame: &mut ExceptionStackFrame, error_code: PageFaultErrorCode) {
    println!("PAGE FAULT!");
    println!("Address: {:x}", control_regs::cr2());
    println!("Error code: {:?}",
             error_code);
    println!("Page flags: {:?}",
             super::memory::get_page_flags(control_regs::cr2().0));
    println!("{:?}", stack_frame);
    loop {}
}

/// The software interrupt handler that invokes schedule operations.
extern "x86-interrupt" fn schedule_interrupt(_: &mut ExceptionStackFrame) {
    lapic::set_priority(0x20);
    lapic::signal_eoi();
    unsafe {
        schedule_next_thread();
        interrupts::disable();
    }
    lapic::set_priority(0x0);
}

/// An interrupt handler that does nothing.
extern "x86-interrupt" fn empty_handler(_: &mut ExceptionStackFrame) {}

irq_interrupt!(
/// The handler for the lapic timer interrupt.
fn timer_handler {
    ::interrupts::timer_interrupt();
});

irq_interrupt!(
/// The handler for IRQ1.
fn irq1_handler {
    let scancode = unsafe { ::x86_64::instructions::port::inb(0x60) };

    ::interrupts::keyboard_interrupt(scancode);
});
