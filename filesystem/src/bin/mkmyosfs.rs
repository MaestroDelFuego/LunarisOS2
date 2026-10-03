use myos_filesystem::{BlockDevice, FileSystem, BLOCK_SIZE, MIN_VOLUME_BLOCKS};
use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

struct ImageFileGuard {
    path: PathBuf,
    keep: bool,
}

impl Drop for ImageFileGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

struct ImageDevice {
    file: File,
    block_count: u32,
}

impl BlockDevice for ImageDevice {
    type Error = io::Error;

    fn block_count(&self) -> u32 {
        self.block_count
    }

    fn read_block(&mut self, block: u32, buffer: &mut [u8; BLOCK_SIZE]) -> Result<(), Self::Error> {
        self.seek_to_block(block)?;
        self.file.read_exact(buffer)
    }

    fn write_block(&mut self, block: u32, buffer: &[u8; BLOCK_SIZE]) -> Result<(), Self::Error> {
        self.seek_to_block(block)?;
        self.file.write_all(buffer)
    }
}

impl ImageDevice {
    fn seek_to_block(&mut self, block: u32) -> io::Result<()> {
        if block >= self.block_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "block is outside the image",
            ));
        }

        self.file
            .seek(SeekFrom::Start(u64::from(block) * BLOCK_SIZE as u64))?;
        Ok(())
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    if arguments.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: mkmyosfs <new-image-path> <block-count>",
        )
        .into());
    }

    let block_count = arguments[1]
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "block count is not text"))?
        .parse::<u32>()?;
    if block_count < MIN_VOLUME_BLOCKS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("block count must be at least {MIN_VOLUME_BLOCKS}"),
        )
        .into());
    }
    let image_size = u64::from(block_count)
        .checked_mul(BLOCK_SIZE as u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "image size overflow"))?;

    let image_path = PathBuf::from(&arguments[0]);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&image_path)?;
    let mut image_guard = ImageFileGuard {
        path: image_path.to_path_buf(),
        keep: false,
    };
    file.set_len(image_size)?;
    let sync_file = file.try_clone()?;

    let image = ImageDevice { file, block_count };
    let filesystem = FileSystem::format(image)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, format!("{error:?}")))?;
    drop(filesystem);
    sync_file.sync_all()?;
    image_guard.keep = true;

    println!(
        "Created MyOS filesystem image {} ({} blocks, {} bytes).",
        image_path.display(),
        block_count,
        image_size
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("mkmyosfs: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::ImageDevice;
    use myos_filesystem::{FileSystem, BLOCK_SIZE, MIN_VOLUME_BLOCKS};
    use std::fs::OpenOptions;
    use std::io;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn new_image_can_mount_and_persist_file_changes() {
        let image_path = std::env::temp_dir().join(format!(
            "myosfs-{}-{}.img",
            std::process::id(),
            NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        create_and_check_image(&image_path).unwrap();
    }

    fn create_and_check_image(image_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let block_count = MIN_VOLUME_BLOCKS + 32;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&image_path)?;
        let _image_guard = super::ImageFileGuard {
            path: image_path.to_path_buf(),
            keep: false,
        };
        file.set_len(u64::from(block_count) * BLOCK_SIZE as u64)?;
        let sync_file = file.try_clone()?;

        let image = ImageDevice { file, block_count };
        let filesystem = FileSystem::format(image).map_err(filesystem_error)?;
        drop(filesystem);
        sync_file.sync_all()?;

        let file = OpenOptions::new().read(true).write(true).open(image_path)?;
        let mut filesystem =
            FileSystem::mount(ImageDevice { file, block_count }).map_err(filesystem_error)?;
        filesystem
            .write_file("NOTE.TXT", b"persistent note")
            .map_err(filesystem_error)?;
        let documents = filesystem
            .create_directory(None, "DOCS")
            .map_err(filesystem_error)?;
        filesystem
            .write_file_in(Some(documents), "NESTED.TXT", b"nested persistent file")
            .map_err(filesystem_error)?;
        drop(filesystem);
        sync_file.sync_all()?;

        let file = OpenOptions::new().read(true).write(true).open(image_path)?;
        let mut filesystem =
            FileSystem::mount(ImageDevice { file, block_count }).map_err(filesystem_error)?;
        let mut contents = [0; 32];
        let length = filesystem
            .read_file("NOTE.TXT", &mut contents)
            .map_err(filesystem_error)?;
        assert_eq!(&contents[..length], b"persistent note");
        let mut entries = [myos_filesystem::DirectoryEntry {
            name: [0; myos_filesystem::MAX_NAME_BYTES],
            name_len: 0,
            size: 0,
            kind: myos_filesystem::EntryKind::File,
            directory_id: None,
        }; 4];
        let count = filesystem
            .list_directory(None, &mut entries)
            .map_err(filesystem_error)?;
        let documents = entries[..count]
            .iter()
            .find(|entry| entry.name() == "DOCS")
            .and_then(|entry| entry.directory_id)
            .ok_or_else(|| io::Error::other("DOCS directory did not survive remount"))?;
        let length = filesystem
            .read_file_in(Some(documents), "NESTED.TXT", &mut contents)
            .map_err(filesystem_error)?;
        assert_eq!(&contents[..length], b"nested persistent file");
        Ok(())
    }

    fn filesystem_error(error: impl core::fmt::Debug) -> io::Error {
        io::Error::new(io::ErrorKind::Other, format!("{error:?}"))
    }
}
