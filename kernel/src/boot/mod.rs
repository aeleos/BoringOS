//! Provides information about the initial status of the system.
mod multiboot;
mod multiboot2;
#[cfg(target_arch = "x86_64")]
use arch::{self, vga_buffer, Architecture};
use core;
use either::Either;
use memory::{Address, MemoryArea, PhysicalAddress, PAGE_SIZE};
/// Lists possiblities for boot sources.
#[derive(PartialEq)]
pub enum BootMethod {
    /// No known bootloader could be found.
    Unknown,
    /// The system was booted using multiboot.
    Multiboot,
    /// The system was booted using multiboot2.
    Multiboot2,
}

/// The memory area containing the initramfs.
fn initramfs() -> MemoryArea<PhysicalAddress> {
    let area = get_initramfs_area();
    // Align to the previous page.
    let initramfs_start = area.start_address().page_align_down();

    // Round up the the next page boundary.
    let initramfs_length = area.length();
    let initramfs_length = if initramfs_length > 0 {
        (initramfs_length - 1) / PAGE_SIZE * PAGE_SIZE + PAGE_SIZE
    } else {
        0
    };
    MemoryArea::new(initramfs_start, initramfs_length)
}

/// Provides an iterator for a memory map.
pub struct MemoryMapIterator<I: Iterator<Item = MemoryArea<PhysicalAddress>>> {
    // multiboot2_iterator: Option<multiboot2::MemoryMapIterator>,
    to_exclude: [MemoryArea<PhysicalAddress>; 2],
    current_entry: Option<MemoryArea<PhysicalAddress>>,
    exclude_index: usize,
    multiboot_iterator: I,
}

impl<I> MemoryMapIterator<I>
where
    I: Iterator<Item = MemoryArea<PhysicalAddress>>,
{
    /// Creates a new memory map iterator.
    fn new(mut iter: I) -> MemoryMapIterator<I> {
        let kernel_area = arch::Current::get_kernel_area();
        let initramfs_area = initramfs();

        let to_exclude = if kernel_area.start_address() <= initramfs_area.start_address() {
            [kernel_area, initramfs_area]
        } else {
            [initramfs_area, kernel_area]
        };

        MemoryMapIterator {
            to_exclude: to_exclude,
            current_entry: iter.next(),
            exclude_index: 0,
            multiboot_iterator: iter,
        }
    }
}

impl<I> Iterator for MemoryMapIterator<I>
where
    I: Iterator<Item = MemoryArea<PhysicalAddress>>,
{
    type Item = MemoryArea<PhysicalAddress>;

    fn next(&mut self) -> Option<MemoryArea<PhysicalAddress>> {
        // NOTE: This assumes function makes a few assumptions to work properly:
        // - The to_exclude list must be ordered by the start addresses.
        // - The to_exclude entries must not overlap.
        // - The memory areas must not overlap.
        // - A to_exclude entry must lie completely within a memory area.

        loop {
            return if let Some(current_entry) = self.current_entry {
                if self.exclude_index >= self.to_exclude.len() {
                    // If all the exclude areas were handled.

                    self.current_entry = self.multiboot_iterator.next();

                    Some(current_entry)
                } else if self.to_exclude[self.exclude_index].is_contained_in(current_entry) {
                    // Handle the exclude areas.

                    // The area to exclude is contained in the current free entry.
                    let (entry_before, entry_after) = {
                        let exclude_area = &self.to_exclude[self.exclude_index];

                        (
                            MemoryArea::new(
                                current_entry.start_address(),
                                exclude_area.start_address() - current_entry.start_address(),
                            ),
                            MemoryArea::new(
                                exclude_area.end_address(),
                                current_entry.end_address() - exclude_area.end_address(),
                            ),
                        )
                    };

                    self.exclude_index += 1;

                    if entry_after.end_address() == entry_after.start_address() {
                        self.current_entry = self.multiboot_iterator.next();
                    } else {
                        self.current_entry = Some(entry_after);
                    }

                    if entry_before.end_address() == entry_before.start_address() {
                        continue;
                    } else {
                        Some(entry_before)
                    }
                } else {
                    self.current_entry = self.multiboot_iterator.next();

                    Some(current_entry)
                }
            } else {
                None
            };
        }
    }
}

/// The method that the system was booted with.
// This will only be set once very early. After that it can be assumed to be
// static.
static mut BOOT_METHOD: BootMethod = BootMethod::Unknown;

/// Initializes the boot module and all the data it provides.
pub fn init(magic_number: u32, information_structure_address: usize) {
    assert_has_not_been_called!("Boot information should only be initialized once.");

    set_boot_method(magic_number);

    match *get_boot_method() {
        BootMethod::Multiboot2 => multiboot2::init(information_structure_address),
        BootMethod::Multiboot => multiboot::init(information_structure_address),
        _ => unimplemented!(),
    };
}

/// Identifies the boot method.
fn set_boot_method(magic_number: u32) {
    unsafe {
        BOOT_METHOD = match magic_number {
            0x36d7_6289 => BootMethod::Multiboot2,
            0x2bad_b002 => BootMethod::Multiboot,
            _ => BootMethod::Unknown,
        }
    }
}

/// Returns the method the system was booted with.
pub fn get_boot_method() -> &'static BootMethod {
    unsafe { &BOOT_METHOD }
}

/// Returns information about the VGA buffer.
#[cfg(target_arch = "x86_64")]
pub fn get_vga_info() -> vga_buffer::Info {
    match *get_boot_method() {
        BootMethod::Multiboot => multiboot::get_vga_info(),
        BootMethod::Multiboot2 => multiboot2::get_vga_info(),
        _ => unimplemented!(),
    }
}

/// Returns the name of the boot loader.
pub fn get_bootloader_name() -> &'static str {
    match *get_boot_method() {
        BootMethod::Multiboot2 => multiboot2::get_bootloader_name(),
        BootMethod::Multiboot => multiboot::get_bootloader_name(),
        _ => "no boot loader",
    }
}

/// Returns the memory area of the initramfs.
pub fn get_initramfs_area() -> MemoryArea<PhysicalAddress> {
    match *get_boot_method() {
        BootMethod::Multiboot => multiboot::get_initramfs_area(),
        BootMethod::Multiboot2 => multiboot2::get_initramfs_area(),
        _ => unimplemented!(),
    }
}

/// Returns an iterator for the map of usable memory.
pub fn get_memory_map() -> Either<
    MemoryMapIterator<multiboot::MemoryMapIterator>,
    MemoryMapIterator<multiboot2::MemoryMapIterator>,
> {
    match *get_boot_method() {
        BootMethod::Multiboot => Either::Left(MemoryMapIterator::new(multiboot::get_memory_map())),
        BootMethod::Multiboot2 => {
            Either::Right(MemoryMapIterator::new(multiboot2::get_memory_map()))
        }
        _ => unimplemented!(),
    }
}

#[repr(C, packed)]
pub struct Multiboot1 {
    magic: u32,
    flags: u32,
    checksum: u32,
    header_addr: u32,
    load_addr: u32,
    load_end_addr: u32,
    bss_end_addr: u32,
    entry_addr: u32,
    mode_type: u32,
    width: u32,
    height: u32,
    depth: u32,
}

#[repr(C, packed)]
struct Multiboot2 {
    pub magic: u32,
    pub arch: u32,
    pub header_length: u32,
    pub checksum: u32,
    pub end_tag_type: u16,
    pub end_tag_flags: u16,
    pub end_tag_size: u32,
}

#[repr(C, align(8))]
pub struct MultibootHeader {
    mb1: Multiboot1,
    mb2: Multiboot2,
}

impl MultibootHeader {
    const MB_MAGIC: u32 = 0x1BADB002;
    const MB_FLAGS: u32 = 0b0000_0000_0000_0000_0000_0000_0000_0000;

    const MB2_MAGIC: u32 = 0xE85250D6;
    const MB2_SIZE: u32 = core::mem::size_of::<Multiboot2>() as u32;

    pub const fn new() -> Self {
        MultibootHeader {
            mb1: Multiboot1 {
                magic: Self::MB_MAGIC,
                flags: Self::MB_FLAGS,
                checksum: u32::max_value() - Self::MB_MAGIC - Self::MB_FLAGS + 1,
                header_addr: 0,
                load_addr: 0,
                load_end_addr: 0,
                bss_end_addr: 0,
                entry_addr: 0,
                mode_type: 0,
                width: 0,
                height: 0,
                depth: 0,
            },
            mb2: Multiboot2 {
                magic: Self::MB2_MAGIC,
                arch: 0,
                header_length: Self::MB2_SIZE,
                checksum: u32::max_value() - Self::MB2_MAGIC - Self::MB2_SIZE + 1,
                end_tag_type: 0,
                end_tag_flags: 0,
                end_tag_size: 8,
            },
        }
    }
}
