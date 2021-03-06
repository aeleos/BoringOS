//! Provides functionality to manage multiple stacks.

use crate::arch::{self, Architecture};
use core::cmp::{max, min};
use core::fmt;
use core::mem::size_of;
use crate::memory::address_space::{AddressSpace, Segment, SegmentType};
use crate::memory::{MemoryArea, PageFlags, VirtualAddress};

// NOTE: For now only full descending stacks are supported.
/// Represents the different types of stacks that exist.
#[allow(dead_code)]
pub enum StackType {
    /// The value currently pointed to is used and the stack grows downward.
    FullDescending,
    /// The value currently pointed to is not used and the stack grows downward.
    EmptyDescending,
    /// The value currently pointed to is used and the stack grows upward.
    FullAscending,
    /// The value currently pointed to is not used and the stack grows upward.
    EmptyAscending
}

/// Determines the type of accesses possible for this stack.
#[derive(PartialEq)]
pub enum AccessType {
    /// The stack can be accessed by usermode code.
    UserAccessible,
    /// The stack can only be accessed by the kernel.
    KernelOnly
}

/// Represents a stack.
pub struct Stack {
    /// Represents the top address of the stack.
    top_address: VirtualAddress,
    /// Represents the bottom address of the stack.
    bottom_address: VirtualAddress,
    /// Represents the maximum stack size.
    max_size: usize,
    /// Represents the first address of the stack.
    pub base_stack_pointer: VirtualAddress,
    /// The access type for this stack.
    access_type: AccessType
}

impl fmt::Debug for Stack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Bottom: {:?}, Top: {:?}, Max size: {:x}",
            self.bottom_address, self.top_address, self.max_size
        )
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        // NOTE: This assumes that the stack is dropped in its own address space.
        self.resize(0, None);
    }
}

impl Stack {
    /// Pushes the given value to the stack pointed to in the given address
    /// space.
    pub fn push_in<T>(
        address_space: &mut AddressSpace,
        stack_pointer: &mut VirtualAddress,
        value: T
    ) {
        match arch::Current::STACK_TYPE {
            StackType::FullDescending => {
                *stack_pointer -= size_of::<T>();
                unsafe {
                    address_space.write_val(value, *stack_pointer);
                }
            },
            _ => unimplemented!("Currently only Full Descending stacks are implemented")
        }
    }

    /// Creates a new stack of size zero with the given start address.
    pub fn new(
        initial_size: usize,
        max_size: usize,
        start_address: VirtualAddress,
        access_type: AccessType,
        mut address_space: Option<&mut AddressSpace>
    ) -> Stack {
        let mut stack = match arch::Current::STACK_TYPE {
            StackType::FullDescending => Stack {
                top_address: start_address + max_size,
                bottom_address: start_address + max_size,
                max_size,
                base_stack_pointer: start_address + max_size,
                access_type
            },
            _ => unimplemented!("Currently only Full Descending stacks are implemented")
        };

        if let Some(ref mut address_space) = address_space {
            let mut flags = PageFlags::READABLE | PageFlags::WRITABLE;

            if stack.access_type == AccessType::UserAccessible {
                flags |= PageFlags::USER_ACCESSIBLE;
            }

            let area = MemoryArea::new(start_address, max_size);

            assert!(
                address_space.add_segment(Segment::new(area, flags, SegmentType::MemoryOnly)),
                "Could not add stack segment."
            );
        }

        stack.resize(initial_size, address_space);

        stack
    }

    /// Grows the stack by the given amount.
    pub fn grow(&mut self, amount: usize, mut address_space: Option<&mut AddressSpace>) {
        match arch::Current::STACK_TYPE {
            StackType::FullDescending => {
                let new_bottom = max(
                    self.top_address - self.max_size,
                    self.bottom_address - amount
                );

                let mut flags = PageFlags::READABLE | PageFlags::WRITABLE;

                if self.access_type == AccessType::UserAccessible {
                    flags |= PageFlags::USER_ACCESSIBLE;
                }

                let first_page_to_map = new_bottom.page_num();

                // This should be one less, but the range is exclusive.
                let last_page_to_map = self.bottom_address.page_num();

                // TODO: flags shouldn't be passed, it should be segment checked instead.
                let mut map_fn = |page_address, flags| match address_space {
                    Some(ref mut address_space) => address_space.map_page(page_address),
                    None => arch::Current::map_page(page_address, flags)
                };

                for page_num in first_page_to_map..last_page_to_map {
                    map_fn(VirtualAddress::from_page_num(page_num), flags);
                }

                self.bottom_address = new_bottom;
            },
            _ => unimplemented!("Currently only Full Descending stacks are implemented")
        }
    }

    /// Shrinks the stack by the given amount.
    pub fn shrink(&mut self, amount: usize, mut address_space: Option<&mut AddressSpace>) {
        match arch::Current::STACK_TYPE {
            StackType::FullDescending => {
                let new_bottom = min(self.top_address, self.bottom_address + amount);

                let first_page_to_unmap = self.bottom_address.page_num();

                // This should be one less, but the range is exclusive.
                let last_page_to_unmap = new_bottom.page_num();

                let mut unmap_fn = |page_address| unsafe {
                    match address_space {
                        Some(ref mut address_space) => address_space.unmap_page(page_address),
                        None => arch::Current::unmap_page(page_address)
                    }
                };

                for page_num in first_page_to_unmap..last_page_to_unmap {
                    unmap_fn(VirtualAddress::from_page_num(page_num));
                }

                self.bottom_address = new_bottom;
            },
            _ => unimplemented!("Currently only Full Descending stacks are implemented")
        }
    }

    /// Resizes the stack to the given size.
    pub fn resize(&mut self, new_size: usize, address_space: Option<&mut AddressSpace>) {
        let current_size = (self.top_address - self.bottom_address) as isize;

        let difference = new_size as isize - current_size;

        if difference > 0 {
            self.grow(difference as usize, address_space);
        } else {
            self.shrink(-difference as usize, address_space);
        }
    }
}
