//! # Memory Management

use {
    alloc::vec::Vec,
    boot_info::{BootInfo, MemoryMap, MemoryRegionKind},
    core::{
        fmt,
        sync::atomic::{AtomicUsize, Ordering},
    },
    linked_list_allocator::LockedHeap,
    log::{debug, error, info, trace, warn},
    memory_types::{Frame, GIBIBYTE, MEBIBYTE, PAGE_SIZE, PhysicalAddress, align_up},
    spin_mutex::Mutex,
    x86_64::{
        VirtAddr,
        instructions::interrupts::without_interrupts,
        registers::control::{Cr0, Cr0Flags, Cr3, Cr3Flags},
        structures::paging::{
            FrameAllocator as ExternFrameAllocator, Mapper as _, OffsetPageTable, Page, PageTable,
            PageTableFlags, PhysFrame, Size4KiB, Translate as _, page::PageRangeInclusive,
        },
    },
};


const HEAP_BASE: usize = 0xFFFF_FE80_0000_0000;

#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::empty();
static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());


pub fn init(boot_info: &BootInfo) {
    info!("Initializing memory management...");

    init_kernel_address_space();
    let addr_space = kernel_address_space();

    assert!(addr_space.is_current());

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

    let free_frames = {
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

        free_frames
    };

    let heap_size = (free_frames * PAGE_SIZE) / 2;
    let heap_start = VirtAddr::new(HEAP_BASE as u64);

    let heap_pages = {
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_start + heap_size as u64 - 1);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Note that we can't use `addr_space.map_pages` here because it requires a heap
    // to already be initialized (internally calls `Vec::push`).
    {
        let mut frame_allocator = FRAME_ALLOCATOR.lock();
        let mut page_table = addr_space.page_table.lock();

        for page in heap_pages {
            let frame = frame_allocator.allocate_frame().unwrap();
            unsafe {
                page_table
                    .map_to(
                        page,
                        frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE,
                        &mut *frame_allocator,
                    )
                    .unwrap()
                    .flush();
            }
        }
    }

    debug!(
        "Initializing heap at {:#x} ({} pages, {} MiB)...",
        addr_space
            .translate_address(heap_start)
            .expect("should be able to translate HEAP_BASE after mapping it"),
        heap_size / PAGE_SIZE,
        heap_size / MEBIBYTE,
    );

    unsafe {
        ALLOCATOR.lock().init(HEAP_BASE, heap_size);
    }

    // Update the kernel mapping offset so kernel mappings start at the right
    // address.
    KERNEL_MAPPING_OFFSET.store(align_up(HEAP_BASE + heap_size, PAGE_SIZE), Ordering::SeqCst);

    info!("Kernel mappings start at {:#x}", HEAP_BASE + heap_size);

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



static KERNEL_MAPPING_OFFSET: AtomicUsize = AtomicUsize::new(0);

/// A set of mapped pages within the kernel's [`AddressSpace`].
#[derive(Debug)]
pub struct KernelMapping {
    pub addr: VirtAddr,
    pub size: usize,
    pub pages: PageRangeInclusive<Size4KiB>,
}

impl KernelMapping {
    pub fn new(size_in_bytes: usize, flags: PageTableFlags) -> Self {
        let addr = VirtAddr::new(KERNEL_MAPPING_OFFSET.fetch_add(
            size_in_bytes.div_ceil(PAGE_SIZE) * PAGE_SIZE,
            Ordering::SeqCst,
        ) as u64);
        let start_page = Page::containing_address(addr);
        let end_page = Page::containing_address(addr + size_in_bytes as u64);
        let pages = Page::range_inclusive(start_page, end_page);

        assert_eq!(pages.count(), size_in_bytes.div_ceil(PAGE_SIZE));

        kernel_address_space().map_pages(pages, flags);

        Self {
            addr,
            size: size_in_bytes,
            pages,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn as_slice_mut(&mut self, offset: usize, len: usize) -> &mut [u8] {
        assert!(kernel_address_space().is_current());
        assert!(
            offset + len <= self.size(),
            "Requested offset and length would overflow kernel mapping",
        );

        let addr = self.addr + offset as u64;

        unsafe { core::slice::from_raw_parts_mut(addr.as_mut_ptr(), len) }
    }
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

unsafe impl ExternFrameAllocator<Size4KiB> for FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.allocate().ok().map(|frame| {
            PhysFrame::containing_address(x86_64::PhysAddr::new(frame.base_addr().to_raw() as u64))
        })
    }
}



fn init_kernel_address_space() {
    unsafe {
        let (l4_frame, _) = Cr3::read();
        let l4_ptr = l4_frame.start_address().as_u64() as *mut PageTable;

        let page_table = OffsetPageTable::new(&mut *l4_ptr, VirtAddr::zero());

        KERNEL_ADDRESS_SPACE = Some(AddressSpace {
            frame: l4_frame,
            frame_allocator: Mutex::new(FrameAllocatorProxy {
                allocated_frames: Vec::new(),
            }),
            page_table: Mutex::new(page_table),
        });
    }
}

pub fn kernel_address_space<'a>() -> &'a AddressSpace {
    unsafe {
        KERNEL_ADDRESS_SPACE
            .as_ref()
            .expect("kernel address space should be initialized")
    }
}

static mut KERNEL_ADDRESS_SPACE: Option<AddressSpace> = None;

pub struct AddressSpace {
    frame: PhysFrame,
    frame_allocator: Mutex<FrameAllocatorProxy>,
    page_table: Mutex<OffsetPageTable<'static>>,
}

impl fmt::Debug for AddressSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AddressSpace @ {:?}", self.frame)
    }
}

impl AddressSpace {
    pub fn new(inherit: Option<&AddressSpace>) -> Self {
        let mut frame_allocator = FrameAllocatorProxy {
            allocated_frames: Vec::new(),
        };
        let frame = frame_allocator
            .allocate_frame()
            .expect("failed to allocate frame for new address space");

        trace!("New address space frame: {frame:?}");

        let page_table = unsafe {
            let l4_ptr = frame.start_address().as_u64() as *mut PageTable;

            OffsetPageTable::new(&mut *l4_ptr, VirtAddr::zero())
        };

        if let Some(parent) = inherit {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    parent.frame.start_address().as_u64() as *const u8,
                    frame.start_address().as_u64() as *mut _,
                    frame.size() as usize,
                );
            }
        }

        Self {
            frame,
            frame_allocator: Mutex::new(frame_allocator),
            page_table: Mutex::new(page_table),
        }
    }

    pub fn is_current(&self) -> bool {
        Cr3::read_raw().0 == self.frame
    }

    pub fn enter(&self) {
        unsafe {
            Cr3::write(self.frame, Cr3Flags::empty());
        }
    }

    pub fn map_pages(&self, pages: PageRangeInclusive, flags: PageTableFlags) {
        let mut frame_allocator = self.frame_allocator.lock();
        let mut page_table = self.page_table.lock();

        for page in pages {
            let frame = frame_allocator.allocate_frame().unwrap();
            // trace!("MAPPING {page:?} TO {frame:?}");
            unsafe {
                page_table
                    .map_to(page, frame, flags, &mut *frame_allocator)
                    .unwrap()
                    .flush();
            }
        }
    }

    pub fn map_kernel_pages_to(
        &self,
        kernel_pages: PageRangeInclusive,
        local_pages: PageRangeInclusive,
        flags: PageTableFlags,
    ) {
        assert_eq!(kernel_pages.count(), local_pages.count());

        let mut frame_allocator = self.frame_allocator.lock();
        let mut page_table = self.page_table.lock();

        for (local_page, kernel_page) in local_pages.zip(kernel_pages) {
            let frame = kernel_address_space()
                .translate_page(kernel_page)
                .expect("should be a mapped kernel page");

            unsafe {
                page_table
                    .map_to(local_page, frame, flags, &mut *frame_allocator)
                    .unwrap()
                    .flush();
            }
        }
    }

    pub fn translate_address(&self, addr: VirtAddr) -> Option<x86_64::PhysAddr> {
        self.page_table.lock().translate_addr(addr)
    }

    pub fn translate_page(&self, page: Page) -> Option<PhysFrame> {
        self.page_table.lock().translate_page(page).ok()
    }
}

/// A proxy to the global [`FrameAllocator`]. Keeps track of allocated frames
/// and deallocates them on drop.
#[derive(Debug)]
pub struct FrameAllocatorProxy {
    allocated_frames: Vec<Frame>,
}

unsafe impl ExternFrameAllocator<Size4KiB> for FrameAllocatorProxy {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = without_interrupts(|| FRAME_ALLOCATOR.lock().allocate_frame());

        if let Some(frame) = frame {
            self.allocated_frames
                .push(Frame::new(frame.start_address().as_u64() as usize));
        }

        frame
    }
}

impl Drop for FrameAllocatorProxy {
    fn drop(&mut self) {
        without_interrupts(|| {
            info!(
                "Dropping frame allocator proxy with {} allocated frames",
                self.allocated_frames.len(),
            );

            let mut global = FRAME_ALLOCATOR.lock();
            for frame in self.allocated_frames.drain(..) {
                let _ = global.deallocate(frame); // Ignore errors.
            }
        });
    }
}
