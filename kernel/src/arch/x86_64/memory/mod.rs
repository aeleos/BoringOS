//! Handles all x86_64 memory related issues.

use crate::memory::{Address, MemoryArea, PageFlags, PhysicalAddress, VirtualAddress};

pub mod address_space_manager;
mod paging;

pub use self::paging::get_free_memory_size;

/// The maximum address of the lower part of the virtual address space.
const VIRTUAL_LOW_MAX_ADDRESS: VirtualAddress = VirtualAddress::from_const(0x0000_7fff_ffff_ffff);

/// The minimum address of the higher part of the virtual address space.
const VIRTUAL_HIGH_MIN_ADDRESS: VirtualAddress = VirtualAddress::from_const(0xffff_8000_0000_0000);

/// The top of the stack after the kernel has been remapped.
pub const FINAL_STACK_TOP: VirtualAddress = VirtualAddress::from_const(0xffff_fe80_0000_0000);

/// The start address for the double fault stack area.
pub const DOUBLE_FAULT_STACK_AREA_BASE: VirtualAddress =
    VirtualAddress::from_const(0xffff_fd00_0000_0000);

/// The distance between two double fault stack tops.
pub const DOUBLE_FAULT_STACK_OFFSET: usize = 0x2000;

/// The maximum size of a double fault stack.
pub const DOUBLE_FAULT_STACK_MAX_SIZE: usize = 0x1000;

/// The base address of the kernel stack area.
pub const KERNEL_STACK_AREA_BASE: VirtualAddress =
    VirtualAddress::from_const(0xffff_fe00_0000_0000);

/// The offset of the start addresses of thread kernel stacks.
pub const KERNEL_STACK_OFFSET: usize = 0x40_0000;

/// The maximum size of a thread kernel stack.
pub const KERNEL_STACK_MAX_SIZE: usize = 0x20_0000;

/// The base address of the process stack area.
pub const USER_STACK_AREA_BASE: VirtualAddress = VirtualAddress::from_const(0x0000_7f80_0000_0000);

/// The offset of the start addresses of thread stacks.
pub const USER_STACK_OFFSET: usize = 0x40_0000;

/// The maximum size of a thread stack.
pub const USER_STACK_MAX_SIZE: usize = 0x20_0000;

/// The start address of the heap.
pub const HEAP_START: VirtualAddress = VirtualAddress::from_const(0xffff_fd80_0000_0000);

/// The maximum size of the heap.
///
/// This is the amount of space a level 3 page table manages.
pub const HEAP_MAX_SIZE: usize = PAGE_SIZE * 512 * 512 * 512;

/// The size of a single page.
pub const PAGE_SIZE: usize = 0x1000;

/// The area where the initramfs will be mapped.
const INITRAMFS_MAP_AREA_START: VirtualAddress =
    VirtualAddress::from_const(0xffff_8000_0000_0000 + 512 * 512 * 512);

/// The run-time memory area of the initramfs.
static mut INITRAMFS_AREA: MemoryArea<VirtualAddress> = MemoryArea::const_default();

extern "C" {
    /// The end of the kernel in its initial mapping.
    static KERNEL_END: PhysicalAddress;
    /// The start of the .text segment.
    static TEXT_START: PhysicalAddress;
    /// The start of the .rodata segment.
    static RODATA_START: PhysicalAddress;
    /// The start of the .data segment.
    static DATA_START: PhysicalAddress;
    /// The start of the .bss segment.
    static BSS_START: PhysicalAddress;
    /// The end of the .bss segment.
    static BSS_END: PhysicalAddress;
    /// The address of the temporary map table.
    static TEMPORARY_MAP_TABLE: PhysicalAddress;
    /// The address of the initial level 4 page table.
    static L4_TABLE: PhysicalAddress;
    /// The address of the initial level 3 page table.
    static L3_TABLE: PhysicalAddress;
    /// The address of the initial level 2 page table.
    static L2_TABLE: PhysicalAddress;
    /// The address of the initial stack level 2 page table.
    static STACK_L2_TABLE: PhysicalAddress;
    /// The address of the initial stack level 1 page table.
    static STACK_L1_TABLE: PhysicalAddress;
    /// The bottom of the initial kernel stack.
    static STACK_BOTTOM: PhysicalAddress;
    /// The top of the initial kernel stack.
    static STACK_TOP: PhysicalAddress;
}

/// The physical address at which the kernel starts.
pub fn get_kernel_area() -> MemoryArea<PhysicalAddress> {
    let start = unsafe { TEXT_START };
    let end = unsafe { KERNEL_END };
    MemoryArea::from_start_and_end(start, end)
}

/// Initializes the memory manager.
pub fn init() {
    assert_has_not_been_called!("The x86_64 memory initialization should only be called once.");

    let physical_initramfs_area = crate::boot::get_initramfs_area();

    paging::init(physical_initramfs_area);

    let start = INITRAMFS_MAP_AREA_START + physical_initramfs_area.start_address().offset_in_page();
    unsafe {
        INITRAMFS_AREA = MemoryArea::new(start, physical_initramfs_area.length());
    }
}

/// Returns the start address of the initramfs.
pub fn get_initramfs_area() -> MemoryArea<VirtualAddress> {
    unsafe { INITRAMFS_AREA }
}

/// Maps the given page using the given flags.
pub fn map_page(page_address: VirtualAddress, flags: PageFlags) {
    paging::map_page(page_address, flags);
}

/// Maps the given page to the given frame using the given flags.
pub fn map_page_at(page_address: VirtualAddress, frame_address: PhysicalAddress, flags: PageFlags) {
    paging::map_page_at(page_address, frame_address, flags);
}

/// Returns the flags of the given page.
pub fn get_page_flags(page_address: VirtualAddress) -> PageFlags {
    paging::get_page_flags(page_address)
}

/// Unmaps the given page.
///
/// # Safety
/// - Make sure that nothing references that page anymore.
pub unsafe fn unmap_page(start_address: VirtualAddress) {
    paging::unmap_page(start_address);
}

/// Checks if the address is a kernel or a userspace address.
pub fn is_userspace_address(address: VirtualAddress) -> bool {
    address <= VIRTUAL_LOW_MAX_ADDRESS
}
