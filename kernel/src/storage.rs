use myos_filesystem::{BlockDevice, FileSystem, FsError, BLOCK_SIZE};
use uefi::boot::{self, ScopedProtocol, SearchType};
use uefi::proto::media::block::BlockIO;

const MAX_IO_ALIGNMENT: usize = 4096;

#[repr(align(4096))]
struct AlignedBlock([u8; BLOCK_SIZE]);

pub struct UefiBlockDevice {
    protocol: ScopedProtocol<BlockIO>,
    media_id: u32,
    block_count: u32,
}

impl UefiBlockDevice {
    fn new(protocol: ScopedProtocol<BlockIO>) -> Option<Self> {
        let media = protocol.media();
        if !media.is_media_present()
            || media.is_logical_partition()
            || media.is_read_only()
            || media.block_size() as usize != BLOCK_SIZE
        {
            return None;
        }

        let alignment = media.io_align().max(1) as usize;
        if alignment > MAX_IO_ALIGNMENT || !alignment.is_power_of_two() {
            return None;
        }

        let block_count = media.last_block().checked_add(1)?;
        let block_count = u32::try_from(block_count).ok()?;
        Some(Self {
            media_id: media.media_id(),
            block_count,
            protocol,
        })
    }
}

impl BlockDevice for UefiBlockDevice {
    type Error = uefi::Error;

    fn block_count(&self) -> u32 {
        self.block_count
    }

    fn read_block(&mut self, block: u32, buffer: &mut [u8; BLOCK_SIZE]) -> Result<(), Self::Error> {
        let mut aligned = AlignedBlock([0; BLOCK_SIZE]);
        self.protocol
            .read_blocks(self.media_id, u64::from(block), &mut aligned.0)?;
        *buffer = aligned.0;
        Ok(())
    }

    fn write_block(&mut self, block: u32, buffer: &[u8; BLOCK_SIZE]) -> Result<(), Self::Error> {
        let aligned = AlignedBlock(*buffer);
        self.protocol
            .write_blocks(self.media_id, u64::from(block), &aligned.0)?;
        self.protocol.flush_blocks()
    }
}

#[derive(Debug)]
pub enum StorageError {
    Locate(uefi::Error),
    Filesystem(FsError<uefi::Error>),
}

pub fn mount_existing() -> Result<Option<FileSystem<UefiBlockDevice>>, StorageError> {
    let handles = boot::locate_handle_buffer(SearchType::from_proto::<BlockIO>())
        .map_err(StorageError::Locate)?;

    for handle in handles.iter() {
        let protocol = match boot::open_protocol_exclusive::<BlockIO>(*handle) {
            Ok(protocol) => protocol,
            Err(error) => {
                uefi::println!(
                    "Skipping UEFI block device: protocol open failed: {:?}",
                    error
                );
                continue;
            }
        };
        let Some(device) = UefiBlockDevice::new(protocol) else {
            continue;
        };

        match FileSystem::mount(device) {
            Ok(filesystem) => return Ok(Some(filesystem)),
            Err(FsError::InvalidVolume | FsError::DeviceTooSmall) => {}
            Err(error) => return Err(StorageError::Filesystem(error)),
        }
    }

    Ok(None)
}
