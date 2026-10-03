#[derive(Clone, Copy, Debug)]
pub struct MemoryLayout {
    pub total_kib: u64,
    pub usable_kib: u64,
}

pub fn detect_memory() -> MemoryLayout {
    // TODO: replace the placeholder values with actual UEFI memory-map scanning
    // once the kernel foundation is stabilized.
    MemoryLayout {
        total_kib: 65536,
        usable_kib: 65536,
    }
}
