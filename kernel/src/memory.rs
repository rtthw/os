//! # Memory Management

use {
    alloc::vec::Vec,
    boot_info::{BootInfo, MemoryMap, MemoryRegionKind},
    linked_list_allocator::LockedHeap,
    log::{debug, error, info, warn},
    memory_types::{Frame, GIBIBYTE, MEBIBYTE, PAGE_SIZE, PhysicalAddress},
    spin_mutex::Mutex,
    x86_64::{
        VirtAddr,
        registers::control::{Cr0, Cr0Flags},
        structures::paging::{
            FrameAllocator as _, Mapper as _, OffsetPageTable, Page, PageTableFlags, PhysFrame,
            Size4KiB, Translate as _,
        },
    },
};


const HEAP_BASE: usize = 0xFFFF_FE80_0000_0000;

#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::empty();
static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());


pub fn init(boot_info: &BootInfo, page_table: &mut OffsetPageTable) {
    info!("Initializing memory management...");

    // Make sure write protection is off so we don't page fault when we try to write
    // to the read-only UEFI page tables.
    unsafe {
        let cr0 = Cr0::read();
        debug!("CR0: {cr0:?}");
        if cr0.contains(Cr0Flags::WRITE_PROTECT) {
            Cr0::write(cr0 & !Cr0Flags::WRITE_PROTECT);
            info!("Cleared CR0.WP");
        }
    }

    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    frame_allocator.init(&boot_info.memory_map);
    frame_allocator.reserve_range(PhysicalAddress::new(0), MEBIBYTE);
    frame_allocator.reserve_range(
        PhysicalAddress::new(boot_info.kernel_start),
        boot_info.kernel_end - boot_info.kernel_start,
    );

    let free_frames = frame_allocator.free_frames;
    let total_frames = frame_allocator.total_frames;
    let free_memory = (free_frames * PAGE_SIZE) / MEBIBYTE;
    let total_memory = (total_frames * PAGE_SIZE) / MEBIBYTE;
    info!(
        "Physical memory allocator initialized\n\
        \tFree frames: {free_frames} / {total_frames}\n\
        \tFree memory: {free_memory} MiB / {total_memory} MiB",
    );

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

    let heap_size = (free_frames * PAGE_SIZE) / 2;
    let heap_start = VirtAddr::new(HEAP_BASE as u64);

    let heap_pages = {
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_start + heap_size as u64 - 1);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in heap_pages {
        let frame = frame_allocator.allocate_frame().unwrap();
        unsafe {
            page_table
                .map_to(
                    page,
                    frame,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                    &mut *frame_allocator,
                )
                .unwrap()
                .flush();
        }
    }

    debug!(
        "Initializing heap at {:#x} ({} pages, {} MiB)...",
        page_table
            .translate_addr(heap_start)
            .expect("should be able to translate HEAP_BASE after mapping it"),
        heap_size / PAGE_SIZE,
        heap_size / MEBIBYTE,
    );

    unsafe {
        ALLOCATOR.lock().init(HEAP_BASE, heap_size);
    }

    // Make sure the heap allocator actually works.
    initial_heap_test();

    info!("Heap initialized successfully");
}

fn initial_heap_test() {
    {
        let object_1: Vec<u8> = vec![1, 2, 3];
        let object_1_addr = object_1.as_ptr().addr();

        assert!(object_1_addr == HEAP_BASE);
    }

    let object_2: Vec<u8> = vec![4, 5, 6];
    let object_2_addr = object_2.as_ptr().addr();

    // If object 1 failed to deallocate, then this would fail.
    assert!(object_2_addr == HEAP_BASE);

    let object_3: Vec<u8> = vec![7, 8, 9];
    let object_3_addr = object_3.as_ptr().addr();

    // The heap should start at `HEAP_START` and grow upwards, so this object should
    // have a higher virtual address.
    assert!(object_3_addr > HEAP_BASE);
}



const MAX_PHYSICAL_MEMORY: usize = 1 * GIBIBYTE;
const MAX_FRAMES: usize = MAX_PHYSICAL_MEMORY / PAGE_SIZE;
const BITMAP_LEN: usize = MAX_FRAMES / 64;

pub struct FrameAllocator {
    bitmap: [u64; BITMAP_LEN],
    total_frames: usize,
    free_frames: usize,
    next_free_hint: usize,
}

impl FrameAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: [0; BITMAP_LEN],
            total_frames: 0,
            free_frames: 0,
            next_free_hint: 0,
        }
    }

    pub fn init(&mut self, memory_map: &MemoryMap) {
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

        (self.bitmap[word_idx] & (1 << bit_idx)) != 0
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
        if self.free_frames == 0 {
            return Err(FrameAllocatorError::OutOfMemory);
        }

        let start_word = self.next_free_hint / 64;
        for word_index in start_word..BITMAP_LEN {
            if self.bitmap[word_index] != !0 {
                let free_bit = (!self.bitmap[word_index]).trailing_zeros() as usize;
                let frame_num = word_index * 64 + free_bit;

                self.mark_used(frame_num);
                self.free_frames -= 1;
                self.next_free_hint = frame_num + 1;

                return Ok(Frame::new(frame_num));
            }
        }
        for word_index in 0..start_word {
            if self.bitmap[word_index] != !0 {
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameAllocatorError {
    OutOfMemory,
    InvalidFrame,
}

unsafe impl x86_64::structures::paging::FrameAllocator<Size4KiB> for FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.allocate().ok().map(|frame| {
            PhysFrame::containing_address(x86_64::PhysAddr::new(frame.base_addr().to_raw() as u64))
        })
    }
}
