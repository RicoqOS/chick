use x86_64::VirtAddr;
use x86_64::structures::paging::PageTable;

#[derive(Debug, Clone, Copy)]
pub struct MemoryManagement {
    start_address: u64,
}

impl MemoryManagement {
    pub fn new(physical_memory_offset: VirtAddr) -> Self {
        use x86_64::registers::control::Cr3;
        let (page_table, _flags) = Cr3::read();

        let mm = physical_memory_offset + page_table.start_address().as_u64();
        let page_table_ptr: *mut PageTable = mm.as_mut_ptr();

        Self {
            start_address: page_table.start_address().as_u64(),
        }
    }
}
