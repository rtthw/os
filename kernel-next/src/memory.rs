//! # Memory Management

use {
    alloc::vec::Vec,
    boot_info::{BootInfo, MemoryMap, MemoryRegionKind},
    linked_list_allocator::LockedHeap,
    log::{debug, error, info, warn},
    memory_types::{Frame, MEBIBYTE, PAGE_SIZE, PhysicalAddress},
    spin_mutex::Mutex,
};



pub fn init(boot_info: &BootInfo) {
    info!("Initializing memory management...");

    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    frame_allocator.init(&boot_info.memory_map);
    frame_allocator.reserve_range(PhysicalAddress::new(0), MEBIBYTE);
    frame_allocator.reserve_range(
        PhysicalAddress::new(boot_info.kernel_start),
        boot_info.kernel_end - boot_info.kernel_start,
    );

    let free_frames = frame_allocator.free_count();
    let total_frames = frame_allocator.total_count();
    let free_memory = frame_allocator.free_memory() / MEBIBYTE;
    let total_memory = frame_allocator.total_memory() / MEBIBYTE;
    info!("    Physical memory allocator initialized");
    info!("    Free frames: {free_frames} / {total_frames}");
    info!("    Free memory: {free_memory} MiB / {total_memory} MiB");

    match frame_allocator.allocate() {
        Ok(frame) => {
            debug!("    Allocated frame: {frame}");
            if let Err(error) = frame_allocator.deallocate(frame) {
                error!("    Failed to deallocate frame: {error:?}");
            } else {
                debug!("    Deallocated frame successfully");
            }
        }
        Err(error) => {
            error!("    Failed to allocate frame: {error:?}");
        }
    }

    let heap_region = boot_info
        .memory_map
        .iter()
        .filter(|region| region.kind == MemoryRegionKind::Free)
        .max_by_key(|region| region.size)
        .expect("no suitable memory region available for heap");

    debug!(
        "Initializing heap at {:#x}..{:#x} ({} pages, {:#x} bytes)...",
        heap_region.base,
        heap_region.base + heap_region.size,
        heap_region.size / PAGE_SIZE,
        heap_region.size,
    );

    #[allow(static_mut_refs)]
    unsafe {
        ALLOCATOR.lock().init(heap_region.base, heap_region.size);
    }

    // Make sure the heap allocator actually works.
    initial_heap_test(heap_region.base);

    info!("Heap initialized successfully");
}

#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::empty();

fn initial_heap_test(heap_addr: usize) {
    {
        let object_1: Vec<u8> = vec![1, 2, 3];
        let object_1_addr = object_1.as_ptr().addr();

        assert!(object_1_addr == heap_addr);
    }

    let object_2: Vec<u8> = vec![4, 5, 6];
    let object_2_addr = object_2.as_ptr().addr();

    // If object 1 failed to deallocate, then this would fail.
    assert!(object_2_addr == heap_addr);

    let object_3: Vec<u8> = vec![7, 8, 9];
    let object_3_addr = object_3.as_ptr().addr();

    // The heap should start at `heap_addr` and grow upwards, so this object should
    // have a higher address.
    assert!(object_3_addr > heap_addr);
}

pub static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

const MAX_PHYSICAL_MEMORY: u64 = 1 * 1024 * 1024 * 1024;
const MAX_FRAMES: usize = (MAX_PHYSICAL_MEMORY / PAGE_SIZE as u64) as usize;
const BITMAP_WORDS: usize = MAX_FRAMES / 64;

pub struct FrameAllocator {
    bitmap: [u64; BITMAP_WORDS],
    total_frames: usize,
    free_frames: usize,
    initialized: bool,
    next_free_hint: usize,
}

impl FrameAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: [0; BITMAP_WORDS],
            total_frames: 0,
            free_frames: 0,
            initialized: false,
            next_free_hint: 0,
        }
    }

    pub fn init(&mut self, memory_map: &MemoryMap) {
        assert!(!self.initialized);

        for word in self.bitmap.iter_mut() {
            *word = !0;
        }

        self.total_frames = 0;
        self.free_frames = 0;

        for region in memory_map.iter() {
            if region.kind == MemoryRegionKind::Free {
                let start_frame = Frame::containing_addr(PhysicalAddress::new(region.base));
                let end_addr = region.base + region.size;
                let end_frame = Frame::containing_addr(PhysicalAddress::new(end_addr));

                let first_frame = if PhysicalAddress::new(region.base).is_page_aligned() {
                    start_frame.number()
                } else {
                    start_frame.number() + 1
                };
                let last_frame = end_frame.number();

                for frame_num in first_frame..last_frame {
                    if frame_num < MAX_FRAMES {
                        self.mark_free(frame_num);
                        self.total_frames += 1;
                        self.free_frames += 1;
                    }
                }
            }
        }

        self.initialized = true;
        self.next_free_hint = 0;
    }

    pub fn reserve_range(&mut self, base: PhysicalAddress, size: usize) {
        let start_frame = base.frame().number();
        let frame_count = size.div_ceil(PAGE_SIZE);

        for i in 0..frame_count {
            let frame_num = start_frame + i;
            if frame_num < MAX_FRAMES && !self.is_allocated(frame_num) {
                self.mark_used(frame_num);
                if self.free_frames > 0 {
                    self.free_frames -= 1;
                }
            }
        }
    }

    #[inline]
    const fn is_allocated(&self, frame_num: usize) -> bool {
        let word_idx = frame_num / 64;
        let bit_idx = frame_num % 64;

        (self.bitmap[word_idx] & (1u64 << bit_idx)) != 0
    }

    #[inline]
    const fn mark_used(&mut self, frame_num: usize) {
        let word_index = frame_num / 64;
        let bit_index = frame_num % 64;
        self.bitmap[word_index] |= 1 << bit_index;
    }

    #[inline]
    const fn mark_free(&mut self, frame_num: usize) {
        let word_index = frame_num / 64;
        let bit_index = frame_num % 64;
        self.bitmap[word_index] &= !(1 << bit_index);
    }

    pub fn allocate(&mut self) -> Result<Frame, FrameAllocatorError> {
        if !self.initialized {
            return Err(FrameAllocatorError::NotInitialized);
        }

        if self.free_frames == 0 {
            return Err(FrameAllocatorError::OutOfMemory);
        }

        let start_word = self.next_free_hint / 64;
        for word_index in start_word..BITMAP_WORDS {
            if self.bitmap[word_index] != !0u64 {
                let free_bit = (!self.bitmap[word_index]).trailing_zeros() as usize;
                let frame_num = word_index * 64 + free_bit;

                self.mark_used(frame_num);
                self.free_frames -= 1;
                self.next_free_hint = frame_num + 1;

                return Ok(Frame::new(frame_num));
            }
        }
        for word_index in 0..start_word {
            if self.bitmap[word_index] != !0u64 {
                let free_bit = (!self.bitmap[word_index]).trailing_zeros() as usize;
                let frame_num = word_index * 64 + free_bit;

                self.mark_used(frame_num);
                self.free_frames -= 1;
                self.next_free_hint = frame_num + 1;

                return Ok(Frame::new(frame_num));
            }
        }

        Err(FrameAllocatorError::OutOfMemory)
    }

    pub fn deallocate(&mut self, frame: Frame) -> Result<(), FrameAllocatorError> {
        if !self.initialized {
            return Err(FrameAllocatorError::NotInitialized);
        }

        let frame_num = frame.number();

        if frame_num >= MAX_FRAMES {
            return Err(FrameAllocatorError::InvalidFrame);
        }

        if !self.is_allocated(frame_num) {
            warn!("Detected double-free at frame #{frame_num}");
            return Ok(());
        }

        self.mark_free(frame_num);
        self.free_frames += 1;

        if frame_num < self.next_free_hint {
            self.next_free_hint = frame_num;
        }

        Ok(())
    }

    #[inline]
    pub fn free_count(&self) -> usize {
        self.free_frames
    }

    #[inline]
    pub fn total_count(&self) -> usize {
        self.total_frames
    }

    #[inline]
    pub fn free_memory(&self) -> usize {
        self.free_frames * PAGE_SIZE
    }

    #[inline]
    pub fn total_memory(&self) -> usize {
        self.total_frames * PAGE_SIZE
    }
}

/// Errors that can occur during frame allocation
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameAllocatorError {
    OutOfMemory,
    FrameInUse,
    InvalidFrame,
    NotInitialized,
}
