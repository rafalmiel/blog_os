#![allow(dead_code)] // TODO remove

pub mod mapping;

use memory::{PAGE_SIZE, Page, Frame};

pub struct PageTable {
    p4_frame: Frame, // recursive mapped
}

impl PageTable {
    pub fn create_on_identity_mapped_frame(identity_mapped_frame: Frame) -> PageTable {
        {
            // frame must be identity mapped
            let page = Page{number: identity_mapped_frame.number};
            assert!(!page.is_unused());
            assert!(!page.p1_table().entry(511).pointed_frame() == frame);
        }

        let frame_address = identity_mapped_frame.number * PAGE_SIZE;
        let last_entry_address = frame_address + 511 * 8;
        *(last_entry_address as *mut usize) = frame_address | 0b11;
        PageTable {
            p4_frame: identity_mapped_frame,
        }
    }

    pub fn modify<F>(&mut self, f: F) where F: FnOnce(PageTableModifier) {
        let p4_address = 0o177777_777_777_777_777_7770 as *mut usize;
        let backup = unsafe{ *p4_address };
        if Frame::containing_address(backup) == self.p4_frame {
            f(PageTableModifier{_private: ()});
        } else {
            unsafe{ *p4_address = (self.p4_frame.number << 12) | 0b11 };
            f(PageTableModifier{_private: ()});
            unsafe{ *p4_address = backup };
        }
    }
}

pub struct PageTableModifier {
    _private: (),
}

impl Page {
    fn p4_index(&self) -> usize {(self.number >> 27) & 0o777}
    fn p3_index(&self) -> usize {(self.number >> 18) & 0o777}
    fn p2_index(&self) -> usize {(self.number >> 9) & 0o777}
    fn p1_index(&self) -> usize {(self.number >> 0) & 0o777}
}
