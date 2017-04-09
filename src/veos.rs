#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![feature(conservative_impl_trait)]
#![no_std]
#![warn(missing_docs)]

//!The VeOS operating system.
//!
//!This crate contains all of the rust code for VeOS.

extern crate rlibc;
extern crate volatile;
extern crate spin;
#[macro_use]
extern crate bitflags;

#[macro_use]
mod macros;
#[macro_use]
mod io;
mod arch;
mod boot;

///The name of the operating system.
static OS_NAME: &str = "VeOS";

///The main entry point for the operating system.
///
///This is what should get called by the loader.
///#Arguments
///`magic_number` should contain a number that identifies a boot loader. `information_structure_address` is used for boot loaders that pass additional information to the operating system.
#[no_mangle]
pub extern fn main(magic_number: u32, information_structure_address: usize) -> ! {
    boot::init(magic_number, information_structure_address);
    io::init();
    println!("Booted {} using {}...", OS_NAME, boot::get_bootloader_name());
    for entry in boot::get_memory_map() {
        println!("Base: {:x}, Length: {:x}", entry.start, entry.length);
    }

    loop {
    }
}

//TODO: add support for stack unwinding
#[lang = "eh_personality"]
extern fn eh_personality() {
    unimplemented!();
}

///The panic handler.
///
///This function gets called when the operating system panics. It aims to provide as much information as possible.
///The arguments are passed by the compiler, this is not meant to be called manually anywhere, but through the panic! macro.
#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    println!("PANIC! in file '{}' at line {}:", file, line);
    println!("{}", fmt);
    loop {
    }
}
