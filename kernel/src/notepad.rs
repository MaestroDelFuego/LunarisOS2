use myos_filesystem::{
    BlockDevice, DirectoryId, FileInfo, FileSystem, FsError, MAX_FILES, MAX_NAME_BYTES,
};
use myos_gui_core::{TextBuffer, TextBufferError};

const DOCUMENT_NAME: &str = "NOTE.TXT";
const TEXT_CAPACITY: usize = 2048;
const VISIBLE_FILES: usize = 7;
const EMPTY_FILE_INFO: FileInfo = FileInfo {
    name: [0; MAX_NAME_BYTES],
    name_len: 0,
    size: 0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogMode {
    Open,
    SaveAs,
    ConfirmOverwrite,
    ConfirmDiscard,
    ConfirmClose,
}

struct FileDialog {
    mode: FileDialogMode,
    directory: Option<DirectoryId>,
    files: [FileInfo; MAX_FILES],
    file_count: usize,
    selected: Option<usize>,
    visible_start: usize,
    filename: [u8; MAX_NAME_BYTES],
    filename_len: usize,
}

impl FileDialog {
    fn new(mode: FileDialogMode, initial_name: &str, directory: Option<DirectoryId>) -> Self {
        let mut dialog = Self {
            mode,
            directory,
            files: [EMPTY_FILE_INFO; MAX_FILES],
            file_count: 0,
            selected: None,
            visible_start: 0,
            filename: [0; MAX_NAME_BYTES],
            filename_len: 0,
        };
        dialog.set_filename(initial_name);
        dialog
    }

    fn filename(&self) -> &str {
        core::str::from_utf8(&self.filename[..self.filename_len]).unwrap_or("")
    }

    fn set_filename(&mut self, name: &str) {
        let count = name.len().min(MAX_NAME_BYTES);
        self.filename[..count].copy_from_slice(&name.as_bytes()[..count]);
        self.filename[count..].fill(0);
        self.filename_len = count;
    }

    fn select(&mut self, index: usize) -> bool {
        if index >= self.file_count {
            return false;
        }
        self.selected = Some(index);
        self.visible_start = index.saturating_sub(VISIBLE_FILES - 1);
        if matches!(self.mode, FileDialogMode::Open | FileDialogMode::SaveAs) {
            let file = self.files[index];
            let name = core::str::from_utf8(&file.name[..file.name_len as usize]).unwrap_or("");
            self.set_filename(name);
        }
        true
    }

    fn move_selection(&mut self, delta: isize) -> bool {
        if self.file_count == 0 {
            return false;
        }
        let current = self.selected.unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.file_count as isize - 1) as usize;
        self.select(next)
    }

    fn insert_character(&mut self, character: char) -> bool {
        if !character.is_ascii_graphic() && character != ' ' {
            return false;
        }
        if matches!(character, '/' | '\\') || self.filename_len == MAX_NAME_BYTES {
            return false;
        }
        self.filename[self.filename_len] = character as u8;
        self.filename_len += 1;
        self.selected = None;
        true
    }

    fn backspace(&mut self) -> bool {
        if self.filename_len == 0 {
            return false;
        }
        self.filename_len -= 1;
        self.filename[self.filename_len] = 0;
        self.selected = None;
        true
    }
}

pub struct FileDialogView<'a> {
    pub mode: FileDialogMode,
    pub files: &'a [FileInfo],
    pub selected: Option<usize>,
    pub visible_start: usize,
    pub filename: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotepadError<E> {
    StorageUnavailable,
    Filesystem(FsError<E>),
    UnsupportedText(TextBufferError),
}

pub struct Notepad {
    document: TextBuffer<TEXT_CAPACITY>,
    initialized: bool,
    status: &'static str,
    filename: [u8; MAX_NAME_BYTES],
    filename_len: usize,
    dirty: bool,
    close_requested: bool,
    dialog: Option<FileDialog>,
    directory: Option<DirectoryId>,
}

impl Notepad {
    pub fn new() -> Self {
        let mut filename = [0; MAX_NAME_BYTES];
        filename[..DOCUMENT_NAME.len()].copy_from_slice(DOCUMENT_NAME.as_bytes());
        Self {
            document: TextBuffer::new(),
            initialized: false,
            status: "Ready",
            filename,
            filename_len: DOCUMENT_NAME.len(),
            dirty: false,
            close_requested: false,
            dialog: None,
            directory: None,
        }
    }

    pub fn text(&self) -> &str {
        self.document.as_str()
    }

    pub const fn cursor(&self) -> usize {
        self.document.cursor()
    }

    pub const fn status(&self) -> &str {
        self.status
    }

    pub const fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn filename(&self) -> &str {
        core::str::from_utf8(&self.filename[..self.filename_len]).unwrap_or(DOCUMENT_NAME)
    }

    pub fn insert(&mut self, character: char) -> bool {
        if self.dialog.is_some()
            || !(character.is_ascii_graphic() || character == ' ' || character == '\n')
        {
            return false;
        }
        match self.document.insert(character) {
            Ok(()) => {
                self.dirty = true;
                self.status = "Modified";
                true
            }
            Err(TextBufferError::TooLong) => {
                self.status = "Document is full";
                false
            }
            Err(TextBufferError::UnsupportedCharacter) => {
                self.status = "Unsupported character";
                false
            }
        }
    }

    pub fn backspace(&mut self) -> bool {
        let changed = self.document.backspace();
        if changed {
            self.dirty = true;
            self.status = "Modified";
        }
        changed
    }

    pub fn delete(&mut self) -> bool {
        let changed = self.document.delete();
        if changed {
            self.dirty = true;
            self.status = "Modified";
        }
        changed
    }

    pub fn move_left(&mut self) -> bool {
        self.document.move_left()
    }

    pub fn move_right(&mut self) -> bool {
        self.document.move_right()
    }

    pub fn move_up(&mut self) -> bool {
        self.document.move_up()
    }

    pub fn move_down(&mut self) -> bool {
        self.document.move_down()
    }

    pub fn move_home(&mut self) -> bool {
        self.document.move_home()
    }

    pub fn move_end(&mut self) -> bool {
        self.document.move_end()
    }

    pub fn set_cursor(&mut self, position: usize) -> bool {
        if self.dialog.is_some() {
            return false;
        }
        self.document.set_cursor(position)
    }

    pub fn load<D: BlockDevice>(
        &mut self,
        filesystem: Option<&mut FileSystem<D>>,
    ) -> Result<(), NotepadError<D::Error>> {
        let Some(filesystem) = filesystem else {
            self.status = "Open failed: no MyOS filesystem mounted";
            return Err(NotepadError::StorageUnavailable);
        };
        let mut filename = [0; MAX_NAME_BYTES];
        let filename_len = self.filename_len;
        filename[..filename_len].copy_from_slice(&self.filename[..filename_len]);
        let filename = core::str::from_utf8(&filename[..filename_len]).unwrap_or(DOCUMENT_NAME);
        match self.load_named(filesystem, self.directory, filename) {
            Ok(()) => Ok(()),
            Err(NotepadError::Filesystem(FsError::FileNotFound)) => {
                self.document.clear();
                self.initialized = true;
                self.dirty = false;
                self.status = "New document";
                Ok(())
            }
            Err(error) => {
                self.status = "Open failed; see serial log";
                Err(error)
            }
        }
    }

    pub fn save<D: BlockDevice>(
        &mut self,
        filesystem: Option<&mut FileSystem<D>>,
    ) -> Result<(), NotepadError<D::Error>> {
        let Some(filesystem) = filesystem else {
            self.status = "Save failed: no MyOS filesystem mounted";
            return Err(NotepadError::StorageUnavailable);
        };
        let mut filename = [0; MAX_NAME_BYTES];
        let filename_len = self.filename_len;
        filename[..filename_len].copy_from_slice(&self.filename[..filename_len]);
        let filename = core::str::from_utf8(&filename[..filename_len]).unwrap_or(DOCUMENT_NAME);
        let result = self.save_named(filesystem, self.directory, filename);
        if result.is_err() {
            self.status = "Save failed; see serial log";
        }
        result
    }

    pub fn begin_open<D: BlockDevice>(
        &mut self,
        filesystem: Option<&FileSystem<D>>,
    ) -> Result<(), NotepadError<D::Error>> {
        self.begin_open_at(filesystem, self.directory, None)
    }

    pub fn begin_open_at<D: BlockDevice>(
        &mut self,
        filesystem: Option<&FileSystem<D>>,
        directory: Option<DirectoryId>,
        selected_name: Option<&str>,
    ) -> Result<(), NotepadError<D::Error>> {
        let Some(filesystem) = filesystem else {
            self.status = "Open failed: no MyOS filesystem mounted";
            return Err(NotepadError::StorageUnavailable);
        };
        let mut dialog = FileDialog::new(FileDialogMode::Open, self.filename(), directory);
        dialog.file_count = filesystem
            .list_in(directory, &mut dialog.files)
            .map_err(NotepadError::Filesystem)?;
        if dialog.file_count > 0 {
            dialog.select(0);
        }
        if let Some(name) = selected_name {
            let Some(index) = dialog.files[..dialog.file_count]
                .iter()
                .position(|file| file.name() == name)
            else {
                self.status = "Open failed: file is no longer in this folder";
                return Err(NotepadError::Filesystem(FsError::FileNotFound));
            };
            dialog.select(index);
        }
        self.dialog = Some(dialog);
        self.status = "Choose a file; Enter opens, Esc cancels";
        Ok(())
    }

    pub fn begin_save_as<D: BlockDevice>(
        &mut self,
        filesystem: Option<&FileSystem<D>>,
    ) -> Result<(), NotepadError<D::Error>> {
        let Some(filesystem) = filesystem else {
            self.status = "Save As failed: no MyOS filesystem mounted";
            return Err(NotepadError::StorageUnavailable);
        };
        let mut dialog = FileDialog::new(FileDialogMode::SaveAs, self.filename(), self.directory);
        dialog.file_count = filesystem
            .list_in(self.directory, &mut dialog.files)
            .map_err(NotepadError::Filesystem)?;
        self.dialog = Some(dialog);
        self.status = "Enter a name; existing files require confirmation";
        Ok(())
    }

    pub const fn has_dialog(&self) -> bool {
        self.dialog.is_some()
    }

    pub fn request_close(&mut self) -> bool {
        if self.dirty {
            self.dialog = Some(FileDialog::new(
                FileDialogMode::ConfirmClose,
                self.filename(),
                self.directory,
            ));
            self.status = "Discard unsaved changes and close?";
            false
        } else {
            self.close_requested = true;
            true
        }
    }

    pub fn take_close_request(&mut self) -> bool {
        core::mem::replace(&mut self.close_requested, false)
    }

    pub fn dialog_view(&self) -> Option<FileDialogView<'_>> {
        self.dialog.as_ref().map(|dialog| FileDialogView {
            mode: dialog.mode,
            files: &dialog.files[..dialog.file_count],
            selected: dialog.selected,
            visible_start: dialog.visible_start,
            filename: dialog.filename(),
        })
    }

    pub fn dialog_select(&mut self, index: usize) -> bool {
        self.dialog
            .as_mut()
            .is_some_and(|dialog| dialog.select(index))
    }

    pub fn dialog_move_selection(&mut self, delta: isize) -> bool {
        self.dialog.as_mut().is_some_and(|dialog| {
            matches!(dialog.mode, FileDialogMode::Open | FileDialogMode::SaveAs)
                && dialog.move_selection(delta)
        })
    }

    pub fn dialog_insert(&mut self, character: char) -> bool {
        let changed = self.dialog.as_mut().is_some_and(|dialog| {
            dialog.mode == FileDialogMode::SaveAs && dialog.insert_character(character)
        });
        if !changed {
            self.status = "Name character is invalid or the name is full";
        }
        changed
    }

    pub fn dialog_backspace(&mut self) -> bool {
        let changed = self
            .dialog
            .as_mut()
            .is_some_and(|dialog| dialog.mode == FileDialogMode::SaveAs && dialog.backspace());
        if !changed {
            self.status = "Filename is empty";
        }
        changed
    }

    pub fn dialog_cancel(&mut self) -> bool {
        let Some(dialog) = self.dialog.as_mut() else {
            return false;
        };
        match dialog.mode {
            FileDialogMode::ConfirmOverwrite => {
                dialog.mode = FileDialogMode::SaveAs;
                self.status = "Overwrite cancelled";
            }
            FileDialogMode::ConfirmDiscard => {
                dialog.mode = FileDialogMode::Open;
                self.status = "Open cancelled";
            }
            FileDialogMode::ConfirmClose => {
                self.dialog = None;
                self.status = "Close cancelled";
            }
            FileDialogMode::Open | FileDialogMode::SaveAs => {
                self.dialog = None;
                self.status = "Cancelled";
            }
        }
        true
    }

    pub fn dismiss_dialog(&mut self) -> bool {
        if self.dialog.take().is_none() {
            return false;
        }
        self.status = "Cancelled";
        true
    }

    pub fn dialog_accept<D: BlockDevice>(
        &mut self,
        filesystem: Option<&mut FileSystem<D>>,
    ) -> Result<bool, NotepadError<D::Error>> {
        let Some(dialog) = self.dialog.as_ref() else {
            return Ok(false);
        };
        let mode = dialog.mode;
        let directory = dialog.directory;
        if mode == FileDialogMode::ConfirmClose {
            self.dialog = None;
            self.document.clear();
            self.initialized = false;
            self.dirty = false;
            self.status = "Closed without saving";
            self.close_requested = true;
            return Ok(true);
        }
        let mut name_bytes = [0; MAX_NAME_BYTES];
        let name_len;
        if matches!(mode, FileDialogMode::Open | FileDialogMode::ConfirmDiscard) {
            let Some(index) = dialog.selected else {
                self.status = "Select a file first";
                return Ok(false);
            };
            let info = dialog.files[index];
            name_len = info.name_len as usize;
            name_bytes[..name_len].copy_from_slice(&info.name[..name_len]);
        } else {
            name_len = dialog.filename_len;
            name_bytes[..name_len].copy_from_slice(&dialog.filename[..name_len]);
        }
        if matches!(
            mode,
            FileDialogMode::SaveAs | FileDialogMode::ConfirmOverwrite
        ) && name_len == 0
        {
            self.status = "Enter a file name";
            return Ok(false);
        }
        if mode == FileDialogMode::Open && self.dirty {
            self.dialog.as_mut().unwrap().mode = FileDialogMode::ConfirmDiscard;
            self.status = "Discard unsaved changes? Enter confirms, Esc returns";
            return Ok(false);
        }
        let name = core::str::from_utf8(&name_bytes[..name_len]).unwrap_or("");
        if mode == FileDialogMode::SaveAs
            && dialog.files[..dialog.file_count]
                .iter()
                .any(|file| file.name() == name)
        {
            self.dialog.as_mut().unwrap().mode = FileDialogMode::ConfirmOverwrite;
            self.status = "Overwrite this file? Enter confirms, Esc returns";
            return Ok(false);
        }
        let Some(filesystem) = filesystem else {
            return Err(NotepadError::StorageUnavailable);
        };
        let result = match mode {
            FileDialogMode::Open | FileDialogMode::ConfirmDiscard => {
                self.load_named(filesystem, directory, name)
            }
            FileDialogMode::SaveAs | FileDialogMode::ConfirmOverwrite => {
                self.save_named(filesystem, directory, name)
            }
            FileDialogMode::ConfirmClose => unreachable!("close confirmation is handled above"),
        };
        if let Err(error) = result {
            self.status = "File operation failed; see serial log";
            return Err(error);
        }
        self.dirty = false;
        self.dialog = None;
        Ok(true)
    }

    fn load_named<D: BlockDevice>(
        &mut self,
        filesystem: &mut FileSystem<D>,
        directory: Option<DirectoryId>,
        name: &str,
    ) -> Result<(), NotepadError<D::Error>> {
        let mut bytes = [0; TEXT_CAPACITY];
        let size = filesystem
            .read_file_in(directory, name, &mut bytes)
            .map_err(NotepadError::Filesystem)?;
        self.document
            .replace(&bytes[..size])
            .map_err(NotepadError::UnsupportedText)?;
        self.set_current_filename(name);
        self.directory = directory;
        self.initialized = true;
        self.dirty = false;
        self.status = "Opened";
        Ok(())
    }

    fn save_named<D: BlockDevice>(
        &mut self,
        filesystem: &mut FileSystem<D>,
        directory: Option<DirectoryId>,
        name: &str,
    ) -> Result<(), NotepadError<D::Error>> {
        filesystem
            .write_file_in(directory, name, self.document.as_bytes())
            .map_err(NotepadError::Filesystem)?;
        self.set_current_filename(name);
        self.directory = directory;
        self.initialized = true;
        self.dirty = false;
        self.status = "Saved";
        Ok(())
    }

    fn set_current_filename(&mut self, name: &str) {
        self.filename.fill(0);
        self.filename_len = name.len().min(MAX_NAME_BYTES);
        self.filename[..self.filename_len].copy_from_slice(&name.as_bytes()[..self.filename_len]);
    }
}
