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

    let mut active_table = unsafe { RecursivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("no more frames");
        InactivePageTable::new_on_identity_mapped_frame(frame, &mut active_table)
    };

    active_table.with(&mut new_table, |table| {
        for section in elf_sections_tag.sections().filter(|s| s.flags & 0x2 != 0) {
            println!("mapping section at addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
                     section.addr,
                     section.size,
                     section.flags);
            let range = Range {
                start: section.addr as usize,
                end: (section.addr + section.size) as usize,
            };
            for address in range.step_by(PAGE_SIZE) {
                let frame = Frame::containing_address(address);
                let flags = WRITABLE | NO_EXECUTE;
                table.identity_map(frame, flags, allocator);
            }
        }
        println!("...done")
    });
}
