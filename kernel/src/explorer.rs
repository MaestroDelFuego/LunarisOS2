use myos_filesystem::{
    DirectoryEntry, DirectoryId, EntryKind, FileSystem, FsError, MAX_FILES, MAX_NAME_BYTES,
};

const VISIBLE_ROWS: usize = 12;
const EMPTY_DIRECTORY_ENTRY: DirectoryEntry = DirectoryEntry {
    name: [0; MAX_NAME_BYTES],
    name_len: 0,
    size: 0,
    kind: EntryKind::File,
    directory_id: None,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplorerDialog {
    Create,
    CreateDirectory,
    Rename,
    Paste,
    ConfirmDelete,
    Properties,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplorerContextAction {
    New,
    Open,
    Rename,
    Delete,
    Copy,
    Paste,
    Properties,
}

pub struct Explorer {
    entries: [DirectoryEntry; MAX_FILES],
    file_count: usize,
    selected: Option<usize>,
    visible_start: usize,
    last_click: Option<(usize, u64)>,
    dialog: Option<ExplorerDialog>,
    edit_name: [u8; MAX_NAME_BYTES],
    edit_name_len: usize,
    clipboard_name: [u8; MAX_NAME_BYTES],
    clipboard_name_len: usize,
    clipboard_directory: Option<DirectoryId>,
    context_menu: Option<(i32, i32)>,
    current_directory: Option<DirectoryId>,
    parent_directories: [Option<DirectoryId>; MAX_FILES],
    depth: usize,
    location: [u8; (MAX_NAME_BYTES + 1) * MAX_FILES],
    location_len: usize,
    status: &'static str,
}

impl Explorer {
    pub const fn new() -> Self {
        Self {
            entries: [EMPTY_DIRECTORY_ENTRY; MAX_FILES],
            file_count: 0,
            selected: None,
            visible_start: 0,
            last_click: None,
            dialog: None,
            edit_name: [0; MAX_NAME_BYTES],
            edit_name_len: 0,
            clipboard_name: [0; MAX_NAME_BYTES],
            clipboard_name_len: 0,
            clipboard_directory: None,
            context_menu: None,
            current_directory: None,
            parent_directories: [None; MAX_FILES],
            depth: 0,
            location: [b'/'; (MAX_NAME_BYTES + 1) * MAX_FILES],
            location_len: 1,
            status: "Not refreshed",
        }
    }

    pub fn refresh<D>(&mut self, filesystem: Option<&FileSystem<D>>) -> bool
    where
        D: myos_filesystem::BlockDevice,
    {
        let Some(filesystem) = filesystem else {
            self.entries.fill(EMPTY_DIRECTORY_ENTRY);
            self.file_count = 0;
            self.selected = None;
            self.visible_start = 0;
            self.last_click = None;
            self.context_menu = None;
            self.status = "No MyOS filesystem mounted";
            return false;
        };

        match filesystem.list_directory(self.current_directory, &mut self.entries) {
            Ok(count) => self.file_count = count,
            Err(error) => {
                self.entries.fill(EMPTY_DIRECTORY_ENTRY);
                self.file_count = 0;
                self.selected = None;
                self.visible_start = 0;
                self.last_click = None;
                self.status = match error {
                    FsError::UnsupportedVersion => "Folders require a version 2 volume",
                    FsError::NotDirectory => "Folder no longer exists; press F5",
                    FsError::Device(_) => "Directory device read failed",
                    FsError::InvalidVolume | FsError::DeviceTooSmall => {
                        "Directory volume is invalid"
                    }
                    FsError::InvalidName
                    | FsError::NameExists
                    | FsError::FileNotFound
                    | FsError::DirectoryFull
                    | FsError::DiskFull
                    | FsError::BufferTooSmall { .. }
                    | FsError::FileTooLarge
                    | FsError::IsDirectory
                    | FsError::DirectoryNotEmpty => "Directory listing failed",
                };
                return false;
            }
        }
        self.selected = (self.file_count > 0).then_some(0);
        self.visible_start = 0;
        self.last_click = None;
        self.context_menu = None;
        self.status = if self.file_count == 0 {
            "Empty folder; F2 creates a file, F4 creates a folder"
        } else {
            "Double-click folders; Backspace up; F2 new F4 folder F3 rename"
        };
        true
    }

    pub fn entries(&self) -> &[DirectoryEntry] {
        &self.entries[..self.file_count]
    }

    pub fn location(&self) -> &str {
        let start = self.location_len.saturating_sub(64);
        core::str::from_utf8(&self.location[start..self.location_len]).unwrap_or("/")
    }

    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub const fn visible_start(&self) -> usize {
        self.visible_start
    }

    pub const fn status(&self) -> &str {
        self.status
    }

    pub fn set_status(&mut self, status: &'static str) {
        self.status = status;
    }

    pub const fn dialog(&self) -> Option<ExplorerDialog> {
        self.dialog
    }

    pub const fn context_menu(&self) -> Option<(i32, i32)> {
        self.context_menu
    }

    pub fn open_context_menu(&mut self, index: usize, x: i32, y: i32) -> bool {
        if index >= self.file_count {
            return false;
        }
        self.select(index);
        self.context_menu = Some((x, y));
        true
    }

    pub fn close_context_menu(&mut self) -> bool {
        self.context_menu.take().is_some()
    }

    pub fn context_action(&mut self, item: usize) -> Option<ExplorerContextAction> {
        self.close_context_menu();
        Some(match item {
            0 => ExplorerContextAction::New,
            1 => ExplorerContextAction::Open,
            2 => ExplorerContextAction::Rename,
            3 => ExplorerContextAction::Delete,
            4 => ExplorerContextAction::Copy,
            5 => ExplorerContextAction::Paste,
            6 => ExplorerContextAction::Properties,
            _ => return None,
        })
    }

    pub fn edit_name(&self) -> &str {
        core::str::from_utf8(&self.edit_name[..self.edit_name_len]).unwrap_or("")
    }

    pub fn begin_create(&mut self) -> bool {
        self.dialog = Some(ExplorerDialog::Create);
        self.set_edit_name("");
        self.status = "Enter a filename; Enter creates, Esc cancels";
        true
    }

    pub fn begin_create_directory(&mut self) -> bool {
        self.dialog = Some(ExplorerDialog::CreateDirectory);
        self.set_edit_name("");
        self.status = "Enter a folder name; Enter creates, Esc cancels";
        true
    }

    pub fn begin_rename(&mut self) -> bool {
        let Some(entry) = self.selected_entry() else {
            self.status = "Select an item to rename";
            return false;
        };
        self.dialog = Some(ExplorerDialog::Rename);
        let name = entry.name();
        self.set_edit_name(name);
        self.status = "Edit name; Enter renames, Esc cancels";
        true
    }

    pub fn begin_delete(&mut self) -> bool {
        if self.selected_entry().is_none() {
            self.status = "Select an item to delete";
            return false;
        }
        self.dialog = Some(ExplorerDialog::ConfirmDelete);
        self.status = "Delete selected item? Enter confirms, Esc cancels";
        true
    }

    pub fn begin_properties(&mut self) -> bool {
        if self.selected_entry().is_none() {
            self.status = "Select an item to view properties";
            return false;
        }
        self.dialog = Some(ExplorerDialog::Properties);
        self.status = "File properties";
        true
    }

    pub fn begin_copy(&mut self) -> bool {
        let Some(file) = self.selected_entry() else {
            self.status = "Select a file to copy";
            return false;
        };
        if file.kind != EntryKind::File {
            self.status = "Folders cannot be copied yet";
            return false;
        }
        self.clipboard_name = file.name;
        self.clipboard_name_len = file.name_len as usize;
        self.clipboard_directory = self.current_directory;
        self.status = "File copied; select Paste to create a duplicate";
        true
    }

    pub fn begin_paste(&mut self) -> bool {
        if self.clipboard_name_len == 0 {
            self.status = "Copy a file before pasting";
            return false;
        }
        let mut source_name = [0; MAX_NAME_BYTES];
        source_name[..self.clipboard_name_len]
            .copy_from_slice(&self.clipboard_name[..self.clipboard_name_len]);
        let source = core::str::from_utf8(&source_name[..self.clipboard_name_len]).unwrap_or("");
        if !self.suggest_copy_name(source) {
            self.status = "No unique destination filename is available";
            return false;
        }
        self.dialog = Some(ExplorerDialog::Paste);
        self.status = "Enter a destination filename; Enter pastes, Esc cancels";
        true
    }

    pub fn edit_insert(&mut self, character: char) -> bool {
        if self
            .dialog
            .is_none_or(|mode| mode == ExplorerDialog::ConfirmDelete)
        {
            return false;
        }
        if !character.is_ascii_graphic()
            || matches!(character, '/' | '\\')
            || self.edit_name_len == MAX_NAME_BYTES
        {
            self.status = "Invalid character or filename is full";
            return false;
        }
        self.edit_name[self.edit_name_len] = character as u8;
        self.edit_name_len += 1;
        true
    }

    pub fn edit_backspace(&mut self) -> bool {
        if self.edit_name_len == 0 {
            return false;
        }
        self.edit_name_len -= 1;
        self.edit_name[self.edit_name_len] = 0;
        true
    }

    pub fn cancel_dialog(&mut self) -> bool {
        if self.dialog.take().is_none() {
            return false;
        }
        self.status = "Operation cancelled";
        true
    }

    pub fn accept_dialog<D: myos_filesystem::BlockDevice>(
        &mut self,
        filesystem: Option<&mut FileSystem<D>>,
    ) -> Result<bool, FsError<D::Error>> {
        let Some(mode) = self.dialog else {
            return Ok(false);
        };
        if mode == ExplorerDialog::Properties {
            self.dialog = None;
            self.status = "Properties closed";
            return Ok(true);
        }
        let Some(filesystem) = filesystem else {
            self.status = "No MyOS filesystem mounted";
            return Ok(true);
        };

        let result = match mode {
            ExplorerDialog::Create => {
                let name = self.edit_name();
                if name.is_empty() {
                    self.status = "Enter a filename";
                    return Ok(false);
                }
                if self.entries().iter().any(|entry| entry.name() == name) {
                    self.status = "A file with that name already exists";
                    return Ok(false);
                }
                filesystem.write_file_in(self.current_directory, name, &[])
            }
            ExplorerDialog::CreateDirectory => {
                let name = self.edit_name();
                if name.is_empty() {
                    self.status = "Enter a folder name";
                    return Ok(false);
                }
                filesystem
                    .create_directory(self.current_directory, name)
                    .map(|_| ())
            }
            ExplorerDialog::Rename => {
                let Some(entry) = self.selected_entry() else {
                    self.status = "Select an item to rename";
                    return Ok(false);
                };
                let mut old_name = [0; MAX_NAME_BYTES];
                let old_len = entry.name_len as usize;
                old_name[..old_len].copy_from_slice(&entry.name[..old_len]);
                let old_name = core::str::from_utf8(&old_name[..old_len]).unwrap_or("");
                let new_name = self.edit_name();
                if new_name.is_empty() {
                    self.status = "Enter a filename";
                    return Ok(false);
                }
                match entry.kind {
                    EntryKind::File => {
                        filesystem.rename_file_in(self.current_directory, old_name, new_name)
                    }
                    EntryKind::Directory => {
                        filesystem.rename_directory(self.current_directory, old_name, new_name)
                    }
                }
            }
            ExplorerDialog::Paste => {
                let destination_name = self.edit_name();
                if destination_name.is_empty() {
                    self.status = "Enter a filename";
                    return Ok(false);
                }
                filesystem.copy_file_between(
                    self.clipboard_directory,
                    self.clipboard_name(),
                    self.current_directory,
                    destination_name,
                )
            }
            ExplorerDialog::Properties => return Ok(false),
            ExplorerDialog::ConfirmDelete => {
                let Some(entry) = self.selected_entry() else {
                    self.status = "Select an item to delete";
                    return Ok(false);
                };
                let mut name = [0; MAX_NAME_BYTES];
                let name_len = entry.name_len as usize;
                name[..name_len].copy_from_slice(&entry.name[..name_len]);
                let name = core::str::from_utf8(&name[..name_len]).unwrap_or("");
                match entry.kind {
                    EntryKind::File => filesystem.delete_file_in(self.current_directory, name),
                    EntryKind::Directory => {
                        filesystem.delete_directory(self.current_directory, name)
                    }
                }
            }
        };

        if let Err(error) = result {
            self.status = match error {
                FsError::InvalidName => "Invalid filename",
                FsError::NameExists => "A file with that name already exists",
                FsError::FileNotFound => "File no longer exists; press F5",
                FsError::DirectoryFull => "The volume has no free file slots",
                FsError::DiskFull => "The volume is full",
                FsError::FileTooLarge => "File is too large",
                FsError::BufferTooSmall { .. } => "Operation needs a larger buffer",
                FsError::NotDirectory => "The selected entry is not a directory",
                FsError::IsDirectory => "The selected entry is a directory",
                FsError::DirectoryNotEmpty => "Directory is not empty",
                FsError::UnsupportedVersion => "Directories need a version 2 MyOS volume",
                FsError::Device(_) | FsError::InvalidVolume | FsError::DeviceTooSmall => {
                    "Filesystem operation failed; see serial log"
                }
            };
            return Err(error);
        }

        self.dialog = None;
        self.refresh(Some(filesystem));
        self.status = match mode {
            ExplorerDialog::Create => "File created",
            ExplorerDialog::CreateDirectory => "Folder created",
            ExplorerDialog::Rename => "Item renamed",
            ExplorerDialog::Paste => "File pasted",
            ExplorerDialog::ConfirmDelete => "Item deleted",
            ExplorerDialog::Properties => "Properties closed",
        };
        Ok(true)
    }

    fn clipboard_name(&self) -> &str {
        core::str::from_utf8(&self.clipboard_name[..self.clipboard_name_len]).unwrap_or("")
    }

    fn suggest_copy_name(&mut self, source: &str) -> bool {
        let mut candidate = [0; MAX_NAME_BYTES];
        for number in 1..=MAX_FILES + 1 {
            let prefix_len = if number == 1 {
                candidate[..5].copy_from_slice(b"COPY_");
                5
            } else {
                candidate[..4].copy_from_slice(b"COPY");
                let mut digits = [0; 3];
                let mut value = number;
                let mut digit_count = 0;
                while value > 0 {
                    digits[digit_count] = b'0' + (value % 10) as u8;
                    digit_count += 1;
                    value /= 10;
                }
                for index in 0..digit_count {
                    candidate[4 + index] = digits[digit_count - index - 1];
                }
                candidate[4 + digit_count] = b'_';
                5 + digit_count
            };
            let source_len = source.len().min(MAX_NAME_BYTES - prefix_len);
            candidate[prefix_len..prefix_len + source_len]
                .copy_from_slice(&source.as_bytes()[..source_len]);
            let name = core::str::from_utf8(&candidate[..prefix_len + source_len]).unwrap_or("");
            if !self.entries().iter().any(|entry| entry.name() == name) {
                self.set_edit_name(name);
                return true;
            }
        }
        false
    }

    fn set_edit_name(&mut self, name: &str) {
        self.edit_name.fill(0);
        self.edit_name_len = name.len().min(MAX_NAME_BYTES);
        self.edit_name[..self.edit_name_len]
            .copy_from_slice(&name.as_bytes()[..self.edit_name_len]);
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.file_count {
            return false;
        }
        let changed = self.selected != Some(index);
        self.selected = Some(index);
        if index < self.visible_start {
            self.visible_start = index;
        } else if index >= self.visible_start + VISIBLE_ROWS {
            self.visible_start = index + 1 - VISIBLE_ROWS;
        }
        changed
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if self.file_count == 0 {
            return false;
        }
        self.last_click = None;
        let current = self.selected.unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.file_count as isize - 1) as usize;
        self.select(next)
    }

    pub fn selected_entry(&self) -> Option<DirectoryEntry> {
        self.selected
            .and_then(|index| self.entries.get(index).copied())
    }

    pub const fn current_directory(&self) -> Option<DirectoryId> {
        self.current_directory
    }

    pub fn enter_selected_directory<D: myos_filesystem::BlockDevice>(
        &mut self,
        filesystem: Option<&FileSystem<D>>,
    ) -> Result<bool, FsError<D::Error>> {
        let Some(entry) = self.selected_entry() else {
            self.status = "Select a folder to open";
            return Ok(false);
        };
        let Some(directory) = entry.directory_id else {
            self.status = "Selected item is not a folder";
            return Ok(false);
        };
        let Some(filesystem) = filesystem else {
            self.status = "No MyOS filesystem mounted";
            return Ok(true);
        };
        if self.depth == MAX_FILES {
            self.status = "Maximum folder depth reached";
            return Ok(false);
        }
        let count = filesystem.list_directory(Some(directory), &mut self.entries)?;
        self.parent_directories[self.depth] = self.current_directory;
        self.depth += 1;
        self.current_directory = Some(directory);
        let name_len = entry.name_len as usize;
        if self.location_len > 1 {
            self.location[self.location_len] = b'/';
            self.location_len += 1;
        }
        self.location[self.location_len..self.location_len + name_len]
            .copy_from_slice(&entry.name[..name_len]);
        self.location_len += name_len;
        self.file_count = count;
        self.reset_selection();
        self.status = if count == 0 {
            "Empty folder; F2 creates a file"
        } else {
            "Double-click a folder to open; Backspace returns"
        };
        Ok(true)
    }

    pub fn go_parent<D: myos_filesystem::BlockDevice>(
        &mut self,
        filesystem: Option<&FileSystem<D>>,
    ) -> Result<bool, FsError<D::Error>> {
        if self.depth == 0 {
            self.status = "Already at the volume root";
            return Ok(true);
        }
        let Some(filesystem) = filesystem else {
            self.status = "No MyOS filesystem mounted";
            return Ok(true);
        };
        let parent_depth = self.depth - 1;
        let parent = self.parent_directories[parent_depth];
        let count = filesystem.list_directory(parent, &mut self.entries)?;
        self.parent_directories[parent_depth] = None;
        self.depth = parent_depth;
        self.current_directory = parent;
        if let Some(separator) = self.location[..self.location_len]
            .iter()
            .rposition(|byte| *byte == b'/')
        {
            self.location_len = separator.max(1);
        }
        self.file_count = count;
        self.reset_selection();
        self.status = "Returned to parent folder";
        Ok(true)
    }

    fn reset_selection(&mut self) {
        self.selected = (self.file_count > 0).then_some(0);
        self.visible_start = 0;
        self.last_click = None;
        self.context_menu = None;
    }

    pub fn click_row(&mut self, index: usize, tick: u64) -> bool {
        if index >= self.file_count {
            self.last_click = None;
            return false;
        }
        self.select(index);
        let double_click = self.last_click.is_some_and(|(previous, previous_tick)| {
            previous == index && tick.saturating_sub(previous_tick) <= 50
        });
        self.last_click = (!double_click).then_some((index, tick));
        double_click
    }
}
