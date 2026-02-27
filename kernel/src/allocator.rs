//! # Memory Allocator
//!
//! The current implementation is a simple [fixed-size block allocator].
//!
//! [fixed-size block allocator]: https://os.phil-opp.com/allocator-designs/#fixed-size-block-allocator

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};

use spin::Mutex;



#[global_allocator]
pub static mut ALLOCATOR: Allocator = Allocator::new();

pub struct Allocator {
    inner: Mutex<FixedSizeBlockAllocator>,
}

struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl Allocator {
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        Self {
            inner: Mutex::new(FixedSizeBlockAllocator {
                list_heads: [EMPTY; BLOCK_SIZES.len()],
                fallback_allocator: linked_list_allocator::Heap::empty(),
            }),
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.inner
                .lock()
                .fallback_allocator
                .init(heap_start, heap_size);
        }
    }
}

impl FixedSizeBlockAllocator {
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => core::ptr::null_mut(),
        }
    }
}

const BLOCK_SIZES: [usize; 10] = [8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.inner.lock();
        match block_index(&layout) {
            Some(index) => match allocator.list_heads[index].take() {
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    node as *mut ListNode as *mut u8
                }
                None => {
                    let block_size = BLOCK_SIZES[index];
                    let block_align = block_size;
                    let layout = Layout::from_size_align(block_size, block_align)
                        .expect("valid size/alignment");
                    allocator.fallback_alloc(layout)
                }
            },
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.inner.lock();
        match block_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                assert!(size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(align_of::<ListNode>() <= BLOCK_SIZES[index]);
                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node);
                    allocator.list_heads[index] = Some(&mut *new_node_ptr);
                }
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                unsafe {
                    allocator.fallback_allocator.deallocate(ptr, layout);
                }
            }
        }
    }
}

fn block_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

struct ListNode {
    next: Option<&'static mut ListNode>,
}
