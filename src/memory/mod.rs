use multiboot2;
use x86;

mod area_frame_allocator;
mod paging;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

impl Frame {
    fn containing_address(address: usize) -> Frame {
        Frame{ number: address / PAGE_SIZE }
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    fn containing_address(address: usize) -> Page {
        Page{ number: address / PAGE_SIZE }
    }
}

pub unsafe fn init(boot_info: &multiboot2::BootInformation) {
    init_safe(boot_info)
}

fn init_safe(boot_info: &multiboot2::BootInformation) {
    use self::area_frame_allocator::AreaFrameAllocator;

    let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");
    let elf_sections_tag = boot_info.elf_sections_tag().expect("Memory map tag required");

    let kernel_start = elf_sections_tag.sections().map(|s| s.addr).min().unwrap();
    let kernel_end = elf_sections_tag.sections().map(|s| s.addr + s.size).max().unwrap();

    let multiboot_start = boot_info as *const _ as usize;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);

    println!("kernel start: 0x{:x}, kernel end: 0x{:x}", kernel_start, kernel_end);
    println!("multiboot start: 0x{:x}, multiboot end: 0x{:x}", multiboot_start, multiboot_end);

    let mut frame_allocator = AreaFrameAllocator::new(kernel_start as usize,
        kernel_end as usize, multiboot_start, multiboot_end, memory_map_tag.memory_areas());

    let page_table_frame = Frame::containing_address(unsafe{x86::controlregs::cr3()} as usize);
    let mut page_table = unsafe {
        paging::PageTable::create_on_identity_mapped_frame(page_table_frame)
    };

    page_table.modify(|mut m| {
        let page = Page::containing_address(0o001_002_003_004_0000);
        let flags = paging::mapping::PRESENT | paging::mapping::NO_EXECUTE;
        unsafe{m.identity_map(&page, flags, &mut frame_allocator)};

        for section in elf_sections_tag.sections() {
            let start_page = Page::containing_address(section.addr as usize);
            let end_page = Page::containing_address((section.addr + section.size - 1) as usize);
            for page_number in start_page.number..(end_page.number + 1) {
                let page = Page{ number: page_number };
                unsafe{m.identity_map(&page, flags, &mut frame_allocator)};
            }
        }
    });
}
