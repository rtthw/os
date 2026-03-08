//! # Paging

use x86_64::{
    VirtAddr,
    structures::paging::{Page, PageTableIndex},
};



pub struct PageAllocator {
    level_4_entries: [bool; 512],
}

impl PageAllocator {
    pub fn new() -> Self {
        let mut page_allocator = Self {
            level_4_entries: [false; 512],
        };
        page_allocator.level_4_entries[0] = true;

        page_allocator
    }

    fn get_free_entries(&mut self, count: usize) -> PageTableIndex {
        // Get available P4 indices with `count` contiguous free entries.
        let mut free_entries = self
            .level_4_entries
            .windows(count)
            .enumerate()
            .filter(|(_, entries)| entries.iter().all(|used| !used))
            .map(|(index, _)| index);

        let index = free_entries
            .next()
            .expect("no usable level 4 entries found");

        // Mark the entries as used.
        for i in 0..count {
            self.level_4_entries[index + i] = true;
        }

        PageTableIndex::new(
            index
                .try_into()
                .expect("page table index larger than u16::MAX"),
        )
    }

    pub fn get_free_address(&mut self, len: usize) -> VirtAddr {
        const LEVEL_4_SIZE: usize = 4096 * 512 * 512 * 512;
        let level_4_entry_count = (len + (LEVEL_4_SIZE - 1)) / LEVEL_4_SIZE;

        Page::from_page_table_indices_1gib(
            self.get_free_entries(level_4_entry_count),
            PageTableIndex::new(0),
        )
        .start_address()
    }
}
