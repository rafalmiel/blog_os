use memory::{PAGE_SIZE, Page, Frame, FrameAllocator};
use memory::paging::PageTableModifier;
use core::intrinsics::offset;
use core::mem::size_of;
use x86::tlb;

impl PageTableModifier {
    pub fn map<A>(&mut self, page: &Page, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
        let frame = allocator.allocate_frame().expect("out of memory");
        self.map_to(page, frame, flags, allocator)
    }

    pub fn map_to<A>(&mut self, page: &Page, frame: Frame, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocator
    {
        let p4_entry = page.p4_table().entry(page.p4_index());
        if p4_entry.is_unused() {
            p4_entry.set(allocator.allocate_frame().expect("no more frames"), PRESENT | WRITABLE);
            unsafe{page.p3_table().zero()};
        }
        let p3_entry = page.p3_table().entry(page.p3_index());
        if p3_entry.is_unused() {
            p3_entry.set(allocator.allocate_frame().expect("no more frames"), PRESENT | WRITABLE);
            unsafe{page.p2_table().zero()};
        }
        let p2_entry = page.p2_table().entry(page.p2_index());
        if p2_entry.is_unused() {
            p2_entry.set(allocator.allocate_frame().expect("no more frames"), PRESENT | WRITABLE);
            unsafe{page.p1_table().zero()};
        }
        let p1_entry = page.p1_table().entry(page.p1_index());
        assert!(p1_entry.is_unused());
        p1_entry.set(frame, flags);
    }

    pub fn identity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocator
    {
        let page = Page{ number: frame.number };
        self.map_to(&page, frame, flags, allocator)
    }


    fn unmap<A>(page: &Page, allocator: &mut A) where A: FrameAllocator {
        assert!(!page.is_unused());
        let p1_entry = page.p1_table().entry(page.p1_index());
        let frame = p1_entry.pointed_frame();
        p1_entry.set_unused();
        unsafe { tlb::flush(page.pointer() as usize) };
        // TODO free p(1,2,3) table if empty
        allocator.deallocate_frame(frame);
    }
}

impl Page {
    pub fn pointer(&self) -> *const () {
        if self.number >= 0o400_000_000_000 {
            //sign extension
            ((self.number << 12) | 0o177777_000_000_000_000_0000) as *const ()
        } else {
            (self.number << 12) as *const ()
        }
    }

    pub fn is_unused(&self) -> bool {
        self.p4_table().entry(self.p4_index()).is_unused() ||
        self.p3_table().entry(self.p3_index()).is_unused() ||
        self.p2_table().entry(self.p2_index()).is_unused() ||
        self.p1_table().entry(self.p1_index()).is_unused()
    }


    fn p4_table(&self) -> Table {
        const P4: Table = Table( Page{ number: 0o_777_777_777_777} );
        P4
    }
    fn p3_table(&self) -> Table {
        Table(Page {
            number: 0o_777_777_777_000 | self.p4_index(),
        })
    }
    fn p2_table(&self) -> Table {
        Table(Page {
            number: 0o_777_777_000_000 | (self.p4_index() << 9) | self.p3_index(),
        })
    }
    fn p1_table(&self) -> Table {
        Table(Page {
            number: 0o_777_000_000_000 | (self.p4_index() << 18) | (self.p3_index() << 9)
                | self.p2_index(),
        })
    }
}

/// A page table on a _mapped_ page.
struct Table(Page);

impl Table {
    unsafe fn zero(&mut self) {
        const ENTRIES: usize = PAGE_SIZE / 8;
        let page = self.0.pointer() as *mut () as *mut [u64; ENTRIES];
        *page = [0; ENTRIES];
    }

    fn entry(&self, index: usize) -> &'static mut TableEntry {
        assert!(index < PAGE_SIZE / size_of::<u64>());
        unsafe {
            let entry = offset(self.0.pointer() as *const u64, index as isize);
            &mut *(entry as *const _ as *mut _)
        }
    }
}

struct TableEntry(u64);

impl TableEntry {
    fn is_unused(&self) -> bool {
        self.0 == 0
    }

    fn set_unused(&mut self) {
        self.0 = 0
    }

    fn set(&mut self, frame: Frame, flags: EntryFlags) {
        self.0 = (((frame.number as u64) << 12) & 0x000fffff_fffff000) | flags.bits();
    }

    fn pointed_frame(&self) -> Frame {
        Frame {
            number: ((self.0 & 0x000fffff_fffff000) >> 12) as usize,
        }
    }

}

bitflags! {
    flags EntryFlags: u64 {
        const PRESENT =         1 << 0,
        const WRITABLE =        1 << 1,
        const USER_ACCESSIBLE = 1 << 2,
        const WRITE_THROUGH =   1 << 3,
        const NO_CACHE =        1 << 4,
        const ACCESSED =        1 << 5,
        const DIRTY =           1 << 6,
        const OTHER1 =          1 << 9,
        const OTHER2 =          1 << 10,
        const NO_EXECUTE =      1 << 63,
    }
}

impl Frame {
    pub fn is_identity_mapped(&self) -> bool {
        let page = Page{number: self.number};
        !page.is_unused() && page.p1_table().entry(page.p1_index()).pointed_frame() == *self
    }
}
