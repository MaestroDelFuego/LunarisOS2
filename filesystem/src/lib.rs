#![no_std]

use core::str;

pub const BLOCK_SIZE: usize = 512;
pub const MAX_FILES: usize = 32;
pub const MAX_NAME_BYTES: usize = 31;
const DIRECTORY_BLOCKS: u32 = 4;
const DATA_START: u32 = 1 + DIRECTORY_BLOCKS;
pub const MIN_VOLUME_BLOCKS: u32 = DATA_START + 1;
const ENTRY_SIZE: usize = 48;
const ENTRIES_PER_BLOCK: usize = BLOCK_SIZE / ENTRY_SIZE;
const DIRECTORY_ENTRIES: usize = ENTRIES_PER_BLOCK * DIRECTORY_BLOCKS as usize;
const _: () = assert!(MAX_FILES <= DIRECTORY_ENTRIES);
const MAGIC_V1: &[u8; 8] = b"MYOSFS1\0";
const MAGIC_V2: &[u8; 8] = b"MYOSFS2\0";
const VERSION_V1: u32 = 1;
const VERSION_V2: u32 = 2;
const ROOT_PARENT: u8 = u8::MAX;

pub trait BlockDevice {
    type Error;

    fn block_count(&self) -> u32;
    fn read_block(&mut self, block: u32, buffer: &mut [u8; BLOCK_SIZE]) -> Result<(), Self::Error>;
    fn write_block(&mut self, block: u32, buffer: &[u8; BLOCK_SIZE]) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FsError<E> {
    Device(E),
    InvalidVolume,
    DeviceTooSmall,
    InvalidName,
    NameExists,
    FileNotFound,
    DirectoryFull,
    DiskFull,
    BufferTooSmall { required: usize },
    FileTooLarge,
    NotDirectory,
    IsDirectory,
    DirectoryNotEmpty,
    UnsupportedVersion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileInfo {
    pub name: [u8; MAX_NAME_BYTES],
    pub name_len: u8,
    pub size: u32,
}

impl FileInfo {
    pub fn name(&self) -> &str {
        str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryId(u8, u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    File,
    Directory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryEntry {
    pub name: [u8; MAX_NAME_BYTES],
    pub name_len: u8,
    pub size: u32,
    pub kind: EntryKind,
    pub directory_id: Option<DirectoryId>,
}

impl DirectoryEntry {
    pub fn name(&self) -> &str {
        str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }
}

#[derive(Clone, Copy)]
struct Entry {
    name: [u8; MAX_NAME_BYTES],
    name_len: u8,
    parent: u8,
    generation: u16,
    is_directory: bool,
    start_block: u32,
    block_len: u32,
    size: u32,
}

impl Entry {
    const EMPTY: Self = Self {
        name: [0; MAX_NAME_BYTES],
        name_len: 0,
        parent: ROOT_PARENT,
        generation: 0,
        is_directory: false,
        start_block: 0,
        block_len: 0,
        size: 0,
    };

    fn is_empty(&self) -> bool {
        self.name_len == 0
    }

    fn info(&self) -> FileInfo {
        FileInfo {
            name: self.name,
            name_len: self.name_len,
            size: self.size,
        }
    }

    fn directory_info(&self, index: usize) -> DirectoryEntry {
        DirectoryEntry {
            name: self.name,
            name_len: self.name_len,
            size: self.size,
            kind: if self.is_directory {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            directory_id: self
                .is_directory
                .then_some(DirectoryId(index as u8, self.generation)),
        }
    }
}

pub struct FileSystem<D> {
    device: D,
    entries: [Entry; MAX_FILES],
    device_blocks: u32,
    version: u32,
}

impl<D: BlockDevice> FileSystem<D> {
    pub fn format(mut device: D) -> Result<Self, FsError<D::Error>> {
        let device_blocks = device.block_count();
        if device_blocks < MIN_VOLUME_BLOCKS {
            return Err(FsError::DeviceTooSmall);
        }

        let mut superblock = [0u8; BLOCK_SIZE];
        superblock[..MAGIC_V2.len()].copy_from_slice(MAGIC_V2);
        put_u32(&mut superblock, 8, VERSION_V2);
        put_u32(&mut superblock, 12, device_blocks);
        put_u32(&mut superblock, 16, DATA_START);
        device
            .write_block(0, &superblock)
            .map_err(FsError::Device)?;

        let empty = [0u8; BLOCK_SIZE];
        for block in 1..DATA_START {
            device.write_block(block, &empty).map_err(FsError::Device)?;
        }

        Ok(Self {
            device,
            entries: [Entry::EMPTY; MAX_FILES],
            device_blocks,
            version: VERSION_V2,
        })
    }

    pub fn mount(mut device: D) -> Result<Self, FsError<D::Error>> {
        let device_blocks = device.block_count();
        if device_blocks < MIN_VOLUME_BLOCKS {
            return Err(FsError::DeviceTooSmall);
        }

        let mut superblock = [0u8; BLOCK_SIZE];
        device
            .read_block(0, &mut superblock)
            .map_err(FsError::Device)?;
        let version = get_u32(&superblock, 8);
        let valid_signature = match version {
            VERSION_V1 => &superblock[..8] == MAGIC_V1,
            VERSION_V2 => &superblock[..8] == MAGIC_V2,
            _ => false,
        };
        if !valid_signature
            || get_u32(&superblock, 12) != device_blocks
            || get_u32(&superblock, 16) != DATA_START
        {
            return Err(FsError::InvalidVolume);
        }

        let mut entries = [Entry::EMPTY; MAX_FILES];
        let mut decoded = 0;
        for block_index in 0..DIRECTORY_BLOCKS {
            let mut block = [0u8; BLOCK_SIZE];
            device
                .read_block(1 + block_index, &mut block)
                .map_err(FsError::Device)?;
            for index in 0..ENTRIES_PER_BLOCK {
                if decoded == MAX_FILES {
                    break;
                }
                let offset = index * ENTRY_SIZE;
                entries[decoded] = if version == VERSION_V1 {
                    decode_entry_v1(&block[offset..offset + ENTRY_SIZE], device_blocks)?
                } else {
                    decode_entry_v2(&block[offset..offset + ENTRY_SIZE], device_blocks)?
                };
                decoded += 1;
            }
        }
        validate_directory_tree(&entries)?;

        Ok(Self {
            device,
            entries,
            device_blocks,
            version,
        })
    }

    pub fn list(&self, output: &mut [FileInfo]) -> usize {
        self.list_directory_files(None, output)
    }

    pub fn list_in(
        &self,
        parent: Option<DirectoryId>,
        output: &mut [FileInfo],
    ) -> Result<usize, FsError<D::Error>> {
        self.validate_parent(parent)?;
        Ok(self.list_directory_files(parent, output))
    }

    fn list_directory_files(&self, parent: Option<DirectoryId>, output: &mut [FileInfo]) -> usize {
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let mut written = 0;
        for entry in self
            .entries
            .iter()
            .filter(|entry| !entry.is_empty() && !entry.is_directory && entry.parent == parent)
        {
            if written == output.len() {
                break;
            }
            output[written] = entry.info();
            written += 1;
        }
        written
    }

    pub fn list_directory(
        &self,
        parent: Option<DirectoryId>,
        output: &mut [DirectoryEntry],
    ) -> Result<usize, FsError<D::Error>> {
        self.validate_parent(parent)?;
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let mut written = 0;
        for (index, entry) in self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.is_empty() && entry.parent == parent)
        {
            if written == output.len() {
                break;
            }
            output[written] = entry.directory_info(index);
            written += 1;
        }
        Ok(written)
    }

    pub fn create_directory(
        &mut self,
        parent: Option<DirectoryId>,
        name: &str,
    ) -> Result<DirectoryId, FsError<D::Error>> {
        self.require_v2()?;
        self.validate_parent(parent)?;
        let name_bytes = validate_name(name)?;
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        if self.find_child(parent, &name_bytes).is_some() {
            return Err(FsError::NameExists);
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.is_empty() && entry.generation < u16::MAX)
            .ok_or(FsError::DirectoryFull)?;
        let generation = self.entries[index].generation + 1;
        let entry = Entry {
            name: name_bytes,
            name_len: name.len() as u8,
            parent,
            generation,
            is_directory: true,
            start_block: 0,
            block_len: 0,
            size: 0,
        };
        self.write_entry(index, entry)?;
        self.entries[index] = entry;
        Ok(DirectoryId(index as u8, generation))
    }

    pub fn delete_directory(
        &mut self,
        parent: Option<DirectoryId>,
        name: &str,
    ) -> Result<(), FsError<D::Error>> {
        self.require_v2()?;
        self.validate_parent(parent)?;
        let name_bytes = validate_name(name)?;
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let index = self
            .find_child(parent, &name_bytes)
            .ok_or(FsError::FileNotFound)?;
        if !self.entries[index].is_directory {
            return Err(FsError::NotDirectory);
        }
        if self
            .entries
            .iter()
            .any(|entry| !entry.is_empty() && entry.parent == index as u8)
        {
            return Err(FsError::DirectoryNotEmpty);
        }
        let mut empty = Entry::EMPTY;
        empty.generation = self.entries[index].generation;
        self.write_entry(index, empty)?;
        self.entries[index] = empty;
        Ok(())
    }

    pub fn rename_directory(
        &mut self,
        parent: Option<DirectoryId>,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), FsError<D::Error>> {
        self.require_v2()?;
        self.validate_parent(parent)?;
        let old_name = validate_name(old_name)?;
        let new_name_bytes = validate_name(new_name)?;
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let index = self
            .find_child(parent, &old_name)
            .ok_or(FsError::FileNotFound)?;
        if !self.entries[index].is_directory {
            return Err(FsError::NotDirectory);
        }
        if old_name != new_name_bytes && self.find_child(parent, &new_name_bytes).is_some() {
            return Err(FsError::NameExists);
        }
        let mut renamed = self.entries[index];
        renamed.name = new_name_bytes;
        renamed.name_len = new_name.len() as u8;
        self.write_entry(index, renamed)?;
        self.entries[index] = renamed;
        Ok(())
    }

    pub fn read_file(&mut self, name: &str, output: &mut [u8]) -> Result<usize, FsError<D::Error>> {
        self.read_file_in(None, name, output)
    }

    pub fn read_file_in(
        &mut self,
        parent: Option<DirectoryId>,
        name: &str,
        output: &mut [u8],
    ) -> Result<usize, FsError<D::Error>> {
        self.validate_parent(parent)?;
        let name_bytes = validate_name(name)?;
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let entry_index = self
            .find_child(parent, &name_bytes)
            .ok_or(FsError::FileNotFound)?;
        if self.entries[entry_index].is_directory {
            return Err(FsError::IsDirectory);
        }
        let entry = self.entries[entry_index];
        let size = entry.size as usize;
        if output.len() < size {
            return Err(FsError::BufferTooSmall { required: size });
        }

        let mut remaining = size;
        let mut copied = 0;
        let mut block_index = entry.start_block;
        while remaining > 0 {
            let mut block = [0u8; BLOCK_SIZE];
            self.device
                .read_block(block_index, &mut block)
                .map_err(FsError::Device)?;
            let count = remaining.min(BLOCK_SIZE);
            output[copied..copied + count].copy_from_slice(&block[..count]);
            copied += count;
            remaining -= count;
            block_index += 1;
        }
        Ok(size)
    }

    pub fn write_file(&mut self, name: &str, contents: &[u8]) -> Result<(), FsError<D::Error>> {
        self.write_file_in(None, name, contents)
    }

    pub fn write_file_in(
        &mut self,
        parent: Option<DirectoryId>,
        name: &str,
        contents: &[u8],
    ) -> Result<(), FsError<D::Error>> {
        self.validate_parent(parent)?;
        let name_bytes = validate_name(name)?;
        let parent_slot = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let child = self.find_child(parent_slot, &name_bytes);
        if child.is_some_and(|index| self.entries[index].is_directory) {
            return Err(FsError::IsDirectory);
        }
        let existing = child;
        let entry_index = match existing {
            Some(index) => index,
            None => self
                .entries
                .iter()
                .position(Entry::is_empty)
                .ok_or(FsError::DirectoryFull)?,
        };

        let blocks_needed = blocks_for(contents.len())?;
        let old = self.entries[entry_index];
        let available = |start: u32| {
            start
                .checked_add(blocks_needed)
                .is_some_and(|end| end <= self.device_blocks)
                && self.entries.iter().enumerate().all(|(index, entry)| {
                    index == entry_index
                        || entry.is_empty()
                        || !ranges_overlap(start, blocks_needed, entry.start_block, entry.block_len)
                })
        };

        let start_block = if blocks_needed == 0 {
            0
        } else if old.block_len >= blocks_needed && available(old.start_block) {
            old.start_block
        } else {
            (DATA_START..self.device_blocks)
                .find(|start| available(*start))
                .ok_or(FsError::DiskFull)?
        };

        let mut written = 0;
        for block_offset in 0..blocks_needed {
            let mut block = [0u8; BLOCK_SIZE];
            let count = (contents.len() - written).min(BLOCK_SIZE);
            block[..count].copy_from_slice(&contents[written..written + count]);
            self.device
                .write_block(start_block + block_offset, &block)
                .map_err(FsError::Device)?;
            written += count;
        }

        let entry = Entry {
            name: name_bytes,
            name_len: name.len() as u8,
            parent: parent_slot,
            generation: old.generation,
            is_directory: false,
            start_block,
            block_len: blocks_needed,
            size: contents.len() as u32,
        };
        self.write_entry(entry_index, entry)?;
        self.entries[entry_index] = entry;
        Ok(())
    }

    pub fn delete_file(&mut self, name: &str) -> Result<(), FsError<D::Error>> {
        self.delete_file_in(None, name)
    }

    pub fn delete_file_in(
        &mut self,
        parent: Option<DirectoryId>,
        name: &str,
    ) -> Result<(), FsError<D::Error>> {
        self.validate_parent(parent)?;
        let name_bytes = validate_name(name)?;
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let entry_index = self
            .find_child(parent, &name_bytes)
            .ok_or(FsError::FileNotFound)?;
        if self.entries[entry_index].is_directory {
            return Err(FsError::IsDirectory);
        }
        let mut empty = Entry::EMPTY;
        empty.generation = self.entries[entry_index].generation;
        self.write_entry(entry_index, empty)?;
        self.entries[entry_index] = empty;
        Ok(())
    }

    pub fn rename_file(&mut self, old_name: &str, new_name: &str) -> Result<(), FsError<D::Error>> {
        self.rename_file_in(None, old_name, new_name)
    }

    pub fn rename_file_in(
        &mut self,
        parent: Option<DirectoryId>,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), FsError<D::Error>> {
        self.validate_parent(parent)?;
        let old_name = validate_name(old_name)?;
        let new_name_bytes = validate_name(new_name)?;
        let parent = parent.map_or(ROOT_PARENT, |directory| directory.0);
        let entry_index = self
            .find_child(parent, &old_name)
            .ok_or(FsError::FileNotFound)?;
        if self.entries[entry_index].is_directory {
            return Err(FsError::IsDirectory);
        }
        if old_name != new_name_bytes && self.find_child(parent, &new_name_bytes).is_some() {
            return Err(FsError::NameExists);
        }

        let mut renamed = self.entries[entry_index];
        renamed.name = new_name_bytes;
        renamed.name_len = new_name.len() as u8;
        self.write_entry(entry_index, renamed)?;
        self.entries[entry_index] = renamed;
        Ok(())
    }

    pub fn copy_file(
        &mut self,
        source_name: &str,
        destination_name: &str,
    ) -> Result<(), FsError<D::Error>> {
        self.copy_file_in(None, source_name, destination_name)
    }

    pub fn copy_file_in(
        &mut self,
        parent: Option<DirectoryId>,
        source_name: &str,
        destination_name: &str,
    ) -> Result<(), FsError<D::Error>> {
        self.copy_file_between(parent, source_name, parent, destination_name)
    }

    pub fn copy_file_between(
        &mut self,
        source_parent: Option<DirectoryId>,
        source_name: &str,
        destination_parent: Option<DirectoryId>,
        destination_name: &str,
    ) -> Result<(), FsError<D::Error>> {
        self.validate_parent(source_parent)?;
        self.validate_parent(destination_parent)?;
        let source_name = validate_name(source_name)?;
        let destination_name_bytes = validate_name(destination_name)?;
        let source_parent = source_parent.map_or(ROOT_PARENT, |directory| directory.0);
        let destination_parent = destination_parent.map_or(ROOT_PARENT, |directory| directory.0);
        let source_index = self
            .find_child(source_parent, &source_name)
            .ok_or(FsError::FileNotFound)?;
        if self.entries[source_index].is_directory {
            return Err(FsError::IsDirectory);
        }
        if self
            .find_child(destination_parent, &destination_name_bytes)
            .is_some()
        {
            return Err(FsError::NameExists);
        }
        let destination_index = self
            .entries
            .iter()
            .position(Entry::is_empty)
            .ok_or(FsError::DirectoryFull)?;
        let source = self.entries[source_index];
        let destination_start = if source.block_len == 0 {
            0
        } else {
            (DATA_START..self.device_blocks)
                .find(|start| {
                    start.checked_add(source.block_len).is_some_and(|end| {
                        end <= self.device_blocks
                            && self.entries.iter().all(|entry| {
                                entry.is_empty()
                                    || !ranges_overlap(
                                        *start,
                                        source.block_len,
                                        entry.start_block,
                                        entry.block_len,
                                    )
                            })
                    })
                })
                .ok_or(FsError::DiskFull)?
        };

        for block_offset in 0..source.block_len {
            let mut block = [0u8; BLOCK_SIZE];
            self.device
                .read_block(source.start_block + block_offset, &mut block)
                .map_err(FsError::Device)?;
            self.device
                .write_block(destination_start + block_offset, &block)
                .map_err(FsError::Device)?;
        }

        let copied = Entry {
            name: destination_name_bytes,
            name_len: destination_name.len() as u8,
            parent: destination_parent,
            generation: self.entries[destination_index].generation,
            is_directory: false,
            start_block: destination_start,
            block_len: source.block_len,
            size: source.size,
        };
        self.write_entry(destination_index, copied)?;
        self.entries[destination_index] = copied;
        Ok(())
    }

    fn find_child(&self, parent: u8, name: &[u8; MAX_NAME_BYTES]) -> Option<usize> {
        self.entries.iter().position(|entry| {
            !entry.is_empty()
                && entry.parent == parent
                && entry.name_len as usize <= MAX_NAME_BYTES
                && entry.name == *name
        })
    }

    fn validate_parent(&self, parent: Option<DirectoryId>) -> Result<(), FsError<D::Error>> {
        let Some(parent) = parent else {
            return Ok(());
        };
        if self.version != VERSION_V2 {
            return Err(FsError::UnsupportedVersion);
        }
        self.entries
            .get(parent.0 as usize)
            .filter(|entry| !entry.is_empty() && entry.is_directory && entry.generation == parent.1)
            .map(|_| ())
            .ok_or(FsError::NotDirectory)
    }

    fn require_v2(&self) -> Result<(), FsError<D::Error>> {
        if self.version == VERSION_V2 {
            Ok(())
        } else {
            Err(FsError::UnsupportedVersion)
        }
    }

    fn write_entry(&mut self, index: usize, entry: Entry) -> Result<(), FsError<D::Error>> {
        let block_index = index / ENTRIES_PER_BLOCK;
        let offset = (index % ENTRIES_PER_BLOCK) * ENTRY_SIZE;
        let mut block = [0u8; BLOCK_SIZE];
        let disk_block = 1 + block_index as u32;
        self.device
            .read_block(disk_block, &mut block)
            .map_err(FsError::Device)?;
        if self.version == VERSION_V1 {
            encode_entry_v1(entry, &mut block[offset..offset + ENTRY_SIZE]);
        } else {
            encode_entry_v2(entry, &mut block[offset..offset + ENTRY_SIZE]);
        }
        self.device
            .write_block(disk_block, &block)
            .map_err(FsError::Device)
    }
}

fn validate_name<E>(name: &str) -> Result<[u8; MAX_NAME_BYTES], FsError<E>> {
    let bytes = name.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_NAME_BYTES
        || !bytes.is_ascii()
        || bytes
            .iter()
            .any(|byte| *byte == b'/' || *byte == b'\\' || *byte == 0)
    {
        return Err(FsError::InvalidName);
    }

    let mut output = [0u8; MAX_NAME_BYTES];
    output[..bytes.len()].copy_from_slice(bytes);
    Ok(output)
}

fn blocks_for<E>(bytes: usize) -> Result<u32, FsError<E>> {
    if bytes > u32::MAX as usize {
        return Err(FsError::FileTooLarge);
    }
    Ok(bytes.div_ceil(BLOCK_SIZE) as u32)
}

fn ranges_overlap(left_start: u32, left_len: u32, right_start: u32, right_len: u32) -> bool {
    left_start < right_start.saturating_add(right_len)
        && right_start < left_start.saturating_add(left_len)
}

fn put_u32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn get_u32(buffer: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap())
}

fn encode_entry_v1(entry: Entry, buffer: &mut [u8]) {
    buffer.fill(0);
    if entry.is_empty() {
        return;
    }
    buffer[0] = 1;
    buffer[1] = entry.name_len;
    buffer[2..2 + MAX_NAME_BYTES].copy_from_slice(&entry.name);
    put_u32(buffer, 36, entry.start_block);
    put_u32(buffer, 40, entry.block_len);
    put_u32(buffer, 44, entry.size);
}

fn encode_entry_v2(entry: Entry, buffer: &mut [u8]) {
    buffer.fill(0);
    buffer[34..36].copy_from_slice(&entry.generation.to_le_bytes());
    if entry.is_empty() {
        return;
    }
    buffer[0] = if entry.is_directory { 2 } else { 1 };
    buffer[1] = entry.name_len;
    buffer[2..2 + MAX_NAME_BYTES].copy_from_slice(&entry.name);
    buffer[33] = entry.parent;
    put_u32(buffer, 36, entry.start_block);
    put_u32(buffer, 40, entry.block_len);
    put_u32(buffer, 44, entry.size);
}

fn decode_name<E>(buffer: &[u8]) -> Result<([u8; MAX_NAME_BYTES], u8), FsError<E>> {
    let name_len = buffer[1] as usize;
    if name_len == 0 || name_len > MAX_NAME_BYTES {
        return Err(FsError::InvalidVolume);
    }
    let mut name = [0u8; MAX_NAME_BYTES];
    name.copy_from_slice(&buffer[2..2 + MAX_NAME_BYTES]);
    if !name[..name_len].is_ascii()
        || name[..name_len]
            .iter()
            .any(|byte| matches!(*byte, b'/' | b'\\' | 0))
    {
        return Err(FsError::InvalidVolume);
    }
    Ok((name, name_len as u8))
}

fn decode_entry_v1<E>(buffer: &[u8], device_blocks: u32) -> Result<Entry, FsError<E>> {
    if buffer[0] == 0 {
        return Ok(Entry::EMPTY);
    }
    if buffer[0] != 1 {
        return Err(FsError::InvalidVolume);
    }
    let (name, name_len) = decode_name(buffer)?;
    let start_block = get_u32(buffer, 36);
    let block_len = get_u32(buffer, 40);
    let size = get_u32(buffer, 44);
    if (block_len == 0 && (start_block != 0 || size != 0))
        || (block_len > 0
            && (start_block < DATA_START
                || start_block.saturating_add(block_len) > device_blocks
                || size > block_len.saturating_mul(BLOCK_SIZE as u32)))
    {
        return Err(FsError::InvalidVolume);
    }
    Ok(Entry {
        name,
        name_len,
        parent: ROOT_PARENT,
        generation: 0,
        is_directory: false,
        start_block,
        block_len,
        size,
    })
}

fn decode_entry_v2<E>(buffer: &[u8], device_blocks: u32) -> Result<Entry, FsError<E>> {
    if buffer[0] == 0 {
        let mut empty = Entry::EMPTY;
        empty.generation = u16::from_le_bytes([buffer[34], buffer[35]]);
        return Ok(empty);
    }
    if !matches!(buffer[0], 1 | 2) {
        return Err(FsError::InvalidVolume);
    }
    let (name, name_len) = decode_name(buffer)?;
    let is_directory = buffer[0] == 2;
    let parent = buffer[33];
    let generation = u16::from_le_bytes([buffer[34], buffer[35]]);
    let start_block = get_u32(buffer, 36);
    let block_len = get_u32(buffer, 40);
    let size = get_u32(buffer, 44);
    if is_directory {
        if generation == 0 || start_block != 0 || block_len != 0 || size != 0 {
            return Err(FsError::InvalidVolume);
        }
    } else if (block_len == 0 && (start_block != 0 || size != 0))
        || (block_len > 0
            && (start_block < DATA_START
                || start_block.saturating_add(block_len) > device_blocks
                || size > block_len.saturating_mul(BLOCK_SIZE as u32)))
    {
        return Err(FsError::InvalidVolume);
    }
    Ok(Entry {
        name,
        name_len,
        parent,
        generation,
        is_directory,
        start_block,
        block_len,
        size,
    })
}

fn validate_directory_tree<E>(entries: &[Entry; MAX_FILES]) -> Result<(), FsError<E>> {
    for (index, entry) in entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| !entry.is_empty())
    {
        if entry.parent != ROOT_PARENT
            && !entries
                .get(entry.parent as usize)
                .is_some_and(|parent| !parent.is_empty() && parent.is_directory)
        {
            return Err(FsError::InvalidVolume);
        }
        if entry.parent == index as u8 {
            return Err(FsError::InvalidVolume);
        }
        if entries[index + 1..].iter().any(|other| {
            !other.is_empty() && other.parent == entry.parent && other.name == entry.name
        }) {
            return Err(FsError::InvalidVolume);
        }

        let mut parent = entry.parent;
        let mut depth = 0;
        while parent != ROOT_PARENT {
            let Some(directory) = entries.get(parent as usize) else {
                return Err(FsError::InvalidVolume);
            };
            if directory.is_empty() || !directory.is_directory {
                return Err(FsError::InvalidVolume);
            }
            parent = directory.parent;
            depth += 1;
            if depth >= MAX_FILES {
                return Err(FsError::InvalidVolume);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::{
        encode_entry_v1, put_u32, BlockDevice, DirectoryEntry, DirectoryId, Entry, EntryKind,
        FileInfo, FileSystem, FsError, BLOCK_SIZE, DATA_START, MAGIC_V1, MAX_FILES, MAX_NAME_BYTES,
        ROOT_PARENT, VERSION_V1,
    };
    use std::vec;
    use std::vec::Vec;

    #[derive(Clone)]
    struct MemoryDisk {
        blocks: Vec<[u8; BLOCK_SIZE]>,
    }

    impl MemoryDisk {
        fn new(blocks: usize) -> Self {
            Self {
                blocks: vec![[0; BLOCK_SIZE]; blocks],
            }
        }
    }

    impl BlockDevice for MemoryDisk {
        type Error = ();

        fn block_count(&self) -> u32 {
            self.blocks.len() as u32
        }

        fn read_block(
            &mut self,
            block: u32,
            buffer: &mut [u8; BLOCK_SIZE],
        ) -> Result<(), Self::Error> {
            *buffer = *self.blocks.get(block as usize).ok_or(())?;
            Ok(())
        }

        fn write_block(
            &mut self,
            block: u32,
            buffer: &[u8; BLOCK_SIZE],
        ) -> Result<(), Self::Error> {
            *self.blocks.get_mut(block as usize).ok_or(())? = *buffer;
            Ok(())
        }
    }

    #[test]
    fn files_survive_unmount_and_remount() {
        let disk = MemoryDisk::new(256);
        let mut fs = FileSystem::format(disk).unwrap();
        fs.write_file("README.TXT", b"MYOS disk-backed files")
            .unwrap();
        fs.write_file("EMPTY", b"").unwrap();
        let disk = fs.device;

        let mut fs = FileSystem::mount(disk).unwrap();
        let mut contents = [0; 64];
        let length = fs.read_file("README.TXT", &mut contents).unwrap();
        assert_eq!(&contents[..length], b"MYOS disk-backed files");
        assert_eq!(fs.read_file("EMPTY", &mut []), Ok(0));
    }

    #[test]
    fn overwrites_and_deletes_files() {
        let disk = MemoryDisk::new(128);
        let mut fs = FileSystem::format(disk).unwrap();
        fs.write_file("NOTE.TXT", b"before").unwrap();
        fs.write_file("NOTE.TXT", b"after and longer").unwrap();

        let mut contents = [0; 32];
        let length = fs.read_file("NOTE.TXT", &mut contents).unwrap();
        assert_eq!(&contents[..length], b"after and longer");
        fs.delete_file("NOTE.TXT").unwrap();
        assert_eq!(
            fs.read_file("NOTE.TXT", &mut contents),
            Err(FsError::FileNotFound)
        );
    }

    #[test]
    fn renames_file_without_changing_contents_and_rejects_duplicates() {
        let disk = MemoryDisk::new(128);
        let mut fs = FileSystem::format(disk).unwrap();
        fs.write_file("OLD.TXT", b"keep this data").unwrap();
        fs.write_file("TAKEN.TXT", b"other").unwrap();

        assert_eq!(
            fs.rename_file("OLD.TXT", "TAKEN.TXT"),
            Err(FsError::NameExists)
        );
        fs.rename_file("OLD.TXT", "NEW.TXT").unwrap();
        let mut contents = [0; 32];
        let length = fs.read_file("NEW.TXT", &mut contents).unwrap();
        assert_eq!(&contents[..length], b"keep this data");
        let disk = fs.device;
        let mut fs = FileSystem::mount(disk).unwrap();
        let length = fs.read_file("NEW.TXT", &mut contents).unwrap();
        assert_eq!(&contents[..length], b"keep this data");
        assert_eq!(
            fs.read_file("OLD.TXT", &mut contents),
            Err(FsError::FileNotFound)
        );
    }

    #[test]
    fn copies_multi_block_and_empty_files_without_overwriting() {
        let disk = MemoryDisk::new(128);
        let mut fs = FileSystem::format(disk).unwrap();
        let contents: Vec<u8> = (0..BLOCK_SIZE + 37).map(|value| value as u8).collect();
        fs.write_file("SOURCE.BIN", &contents).unwrap();
        fs.write_file("EMPTY", b"").unwrap();

        assert_eq!(
            fs.copy_file("SOURCE.BIN", "SOURCE.BIN"),
            Err(FsError::NameExists)
        );
        fs.copy_file("SOURCE.BIN", "COPY.BIN").unwrap();
        fs.copy_file("EMPTY", "EMPTY2").unwrap();
        let mut copied = vec![0; contents.len()];
        let length = fs.read_file("COPY.BIN", &mut copied).unwrap();
        assert_eq!(&copied[..length], contents);
        assert_eq!(fs.read_file("EMPTY2", &mut []), Ok(0));

        let disk = fs.device;
        let mut fs = FileSystem::mount(disk).unwrap();
        let mut copied = vec![0; contents.len()];
        let length = fs.read_file("COPY.BIN", &mut copied).unwrap();
        assert_eq!(&copied[..length], contents);
        let length = fs.read_file("SOURCE.BIN", &mut copied).unwrap();
        assert_eq!(&copied[..length], contents);
    }

    #[test]
    fn copy_reports_disk_full_without_adding_a_directory_entry() {
        let disk = MemoryDisk::new(6);
        let mut fs = FileSystem::format(disk).unwrap();
        fs.write_file("SOURCE", &[7; BLOCK_SIZE]).unwrap();

        assert_eq!(fs.copy_file("SOURCE", "COPY"), Err(FsError::DiskFull));
        let mut entries = [super::FileInfo {
            name: [0; 31],
            name_len: 0,
            size: 0,
        }; 2];
        assert_eq!(fs.list(&mut entries), 1);
        assert_eq!(entries[0].name(), "SOURCE");
    }

    #[test]
    fn reports_name_directory_and_buffer_errors() {
        let disk = MemoryDisk::new(128);
        let mut fs = FileSystem::format(disk).unwrap();
        assert_eq!(fs.write_file("bad/name", b"x"), Err(FsError::InvalidName));
        fs.write_file("ONE", b"1234").unwrap();
        assert_eq!(
            fs.read_file("ONE", &mut [0; 2]),
            Err(FsError::BufferTooSmall { required: 4 })
        );

        for index in 0..MAX_FILES {
            if index == 0 {
                continue;
            }
            let numbered = [
                b'F',
                b'0' + ((index / 10) % 10) as u8,
                b'0' + (index % 10) as u8,
            ];
            let name = core::str::from_utf8(&numbered).unwrap();
            fs.write_file(name, b"").unwrap();
        }
        assert_eq!(fs.write_file("EXTRA", b""), Err(FsError::DirectoryFull));
    }

    #[test]
    fn directory_listing_returns_metadata() {
        let disk = MemoryDisk::new(128);
        let mut fs = FileSystem::format(disk).unwrap();
        fs.write_file("A.TXT", b"abc").unwrap();
        let mut entries = [FileInfo {
            name: [0; 31],
            name_len: 0,
            size: 0,
        }; 4];

        assert_eq!(fs.list(&mut entries), 1);
        assert_eq!(entries[0].name(), "A.TXT");
        assert_eq!(entries[0].size, 3);
    }

    #[test]
    fn directories_isolate_names_and_file_operations_survive_remount() {
        let disk = MemoryDisk::new(256);
        let mut fs = FileSystem::format(disk).unwrap();
        fs.write_file("NOTE.TXT", b"root note").unwrap();
        assert_eq!(
            fs.create_directory(None, "NOTE.TXT"),
            Err(FsError::NameExists)
        );
        let docs = fs.create_directory(None, "DOCS").unwrap();
        let archive = fs.create_directory(Some(docs), "ARCHIVE").unwrap();
        fs.write_file_in(Some(docs), "NOTE.TXT", b"nested note")
            .unwrap();
        fs.write_file_in(Some(archive), "OLD.TXT", b"archived")
            .unwrap();
        fs.rename_directory(Some(docs), "ARCHIVE", "ARCHIVE2")
            .unwrap();

        assert_eq!(fs.create_directory(None, "DOCS"), Err(FsError::NameExists));
        fs.write_file_in(Some(docs), "NOTE.TXT", b"nested note")
            .unwrap();
        assert_eq!(
            fs.create_directory(Some(DirectoryId(u8::MAX - 1, 0)), "BAD"),
            Err(FsError::NotDirectory)
        );
        assert_eq!(
            fs.delete_directory(None, "DOCS"),
            Err(FsError::DirectoryNotEmpty)
        );

        let mut root_entries = [DirectoryEntry {
            name: [0; MAX_NAME_BYTES],
            name_len: 0,
            size: 0,
            kind: EntryKind::File,
            directory_id: None,
        }; 8];
        let count = fs.list_directory(None, &mut root_entries).unwrap();
        assert_eq!(count, 2);
        assert!(root_entries[..count]
            .iter()
            .any(|entry| entry.name() == "DOCS" && entry.kind == EntryKind::Directory));

        let mut nested_entries = [root_entries[0]; 8];
        let count = fs.list_directory(Some(docs), &mut nested_entries).unwrap();
        assert_eq!(count, 2);
        assert!(nested_entries[..count]
            .iter()
            .any(|entry| entry.name() == "NOTE.TXT" && entry.size == 11));
        let mut nested_files = [FileInfo {
            name: [0; MAX_NAME_BYTES],
            name_len: 0,
            size: 0,
        }; 8];
        let count = fs.list_in(Some(docs), &mut nested_files).unwrap();
        assert_eq!(count, 1);
        assert_eq!(nested_files[0].name(), "NOTE.TXT");
        assert_eq!(fs.read_file("DOCS", &mut [0; 1]), Err(FsError::IsDirectory));

        let mut contents = [0; 32];
        let length = fs
            .read_file_in(Some(docs), "NOTE.TXT", &mut contents)
            .unwrap();
        assert_eq!(&contents[..length], b"nested note");
        fs.rename_file_in(Some(docs), "NOTE.TXT", "RENAMED.TXT")
            .unwrap();
        fs.copy_file_in(Some(docs), "RENAMED.TXT", "COPY.TXT")
            .unwrap();
        fs.delete_file_in(Some(docs), "COPY.TXT").unwrap();
        fs.copy_file_between(None, "NOTE.TXT", Some(docs), "ROOT_COPY.TXT")
            .unwrap();
        let copied_length = fs
            .read_file_in(Some(docs), "ROOT_COPY.TXT", &mut contents)
            .unwrap();
        assert_eq!(&contents[..copied_length], b"root note");
        fs.delete_file_in(Some(docs), "ROOT_COPY.TXT").unwrap();

        let disk = fs.device;
        let mut fs = FileSystem::mount(disk).unwrap();
        let mut root_entries = [root_entries[0]; 8];
        let count = fs.list_directory(None, &mut root_entries).unwrap();
        let docs = root_entries[..count]
            .iter()
            .find(|entry| entry.name() == "DOCS")
            .and_then(|entry| entry.directory_id)
            .unwrap();
        let mut contents = [0; 32];
        let length = fs
            .read_file_in(Some(docs), "RENAMED.TXT", &mut contents)
            .unwrap();
        assert_eq!(&contents[..length], b"nested note");
        fs.delete_file_in(Some(docs), "RENAMED.TXT").unwrap();
        fs.delete_file_in(Some(archive), "OLD.TXT").unwrap();
        fs.delete_directory(Some(docs), "ARCHIVE2").unwrap();
        fs.delete_directory(None, "DOCS").unwrap();
        let replacement = fs.create_directory(None, "REPLACEMENT").unwrap();
        assert_ne!(replacement, docs);
        assert_eq!(
            fs.list_directory(Some(docs), &mut nested_entries),
            Err(FsError::NotDirectory)
        );
    }

    #[test]
    fn version_one_volumes_remain_read_write_and_reject_directory_creation() {
        let mut disk = MemoryDisk::new(128);
        let mut superblock = [0; BLOCK_SIZE];
        superblock[..8].copy_from_slice(MAGIC_V1);
        put_u32(&mut superblock, 8, VERSION_V1);
        put_u32(&mut superblock, 12, disk.block_count());
        put_u32(&mut superblock, 16, DATA_START);
        disk.write_block(0, &superblock).unwrap();

        let mut entry_block = [0; BLOCK_SIZE];
        let mut name = [0; MAX_NAME_BYTES];
        name[..10].copy_from_slice(b"LEGACY.TXT");
        let legacy_entry = Entry {
            name,
            name_len: 10,
            parent: ROOT_PARENT,
            generation: 0,
            is_directory: false,
            start_block: 0,
            block_len: 0,
            size: 0,
        };
        encode_entry_v1(legacy_entry, &mut entry_block[..48]);
        disk.write_block(1, &entry_block).unwrap();

        let mut fs = FileSystem::mount(disk).unwrap();
        assert_eq!(
            fs.create_directory(None, "NEW"),
            Err(FsError::UnsupportedVersion)
        );
        fs.write_file("NEW.TXT", b"still writable").unwrap();
        let disk = fs.device;
        let mut fs = FileSystem::mount(disk).unwrap();
        let mut contents = [0; 32];
        let length = fs.read_file("NEW.TXT", &mut contents).unwrap();
        assert_eq!(&contents[..length], b"still writable");
        assert_eq!(
            fs.create_directory(None, "NEW"),
            Err(FsError::UnsupportedVersion)
        );
    }
}
