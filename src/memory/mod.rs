pub use self::area_frame_allocator::AreaFrameAllocator;
pub use self::paging::test_paging;
use self::paging::PhysicalAddress;
use multiboot2::BootInformation;

mod area_frame_allocator;
mod paging;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

impl Frame {
    fn containing_address(address: usize) -> Frame {
        Frame { number: address / PAGE_SIZE }
    }

    fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

struct EmptyFrameAllocator;

impl FrameAllocator for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> { None }
    fn deallocate_frame(&mut self, frame: Frame) {
        unimplemented!();
    }
}

pub fn init<A>(allocator: &mut A, boot_info: &BootInformation)
    where A: FrameAllocator
{
    use self::paging::{RecursivePageTable, InactivePageTable, EntryFlags, WRITABLE, NO_EXECUTE};
    use core::ops::Range;

    let elf_sections_tag = boot_info.elf_sections_tag().expect("Memory map tag required");

    let mut active_table = unsafe { RecursivePageTable::new(allocator) };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("no more frames");
        InactivePageTable::new_on_identity_mapped_frame(frame, &mut active_table)
    };

    active_table.with(&mut new_table, |table| {
        for section in elf_sections_tag.sections() {
            if section.flags & 0x2 == 0 {
                // section is not loaded to memory
                continue
            }

            println!("mapping section at addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
                     section.addr,
                     section.size,
                     section.flags);

            let mut flags = EntryFlags::empty();
            if section.flags & 0x1 != 0 {
                flags = flags | WRITABLE;
            }
            if section.flags & 0x4 == 0 {
                // TODO set NXE bit in EFER
                // flags = flags | NO_EXECUTE;
            }

            let range = Range {
                start: section.addr as usize,
                end: (section.addr + section.size) as usize,
            };
            for address in range.step_by(PAGE_SIZE) {
                let frame = Frame::containing_address(address);
                table.identity_map(frame, flags, allocator);
            }
        }
        println!("...done");
        println!("{:?}", table.translate(0));
        println!("{:?}", table.translate(0xb8000));
        println!("{:?}", table.translate(0x100000));
        println!("{:?}", table.translate(0x10b3e3));
    });

    let old_table = active_table.swap(new_table);
    // TODO unmap old p4 table and turn it into a guard page for the kernel stack
    // TODO make Page Copy and Page::containing_address public?
    //active_table.unmap(Page::containing_address(old_table.start_address()));
}
