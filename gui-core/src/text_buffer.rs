#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextBufferError {
    TooLong,
    UnsupportedCharacter,
}

pub fn visual_cursor(text: &str, cursor: usize, columns: usize) -> (usize, usize) {
    let columns = columns.max(1);
    let mut row = 0;
    let mut column = 0;
    for byte in text.as_bytes().iter().take(cursor.min(text.len())) {
        if *byte == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
            if column == columns {
                row += 1;
                column = 0;
            }
        }
    }
    (row, column)
}

pub fn cursor_at_visual(text: &str, row: usize, column: usize, columns: usize) -> usize {
    let columns = columns.max(1);
    let mut visual_row = 0;
    let mut byte_start = 0;
    for line in text.split('\n') {
        if line.is_empty() {
            if visual_row == row {
                return byte_start.min(text.len());
            }
            visual_row += 1;
        } else {
            for (chunk_index, chunk) in line.as_bytes().chunks(columns).enumerate() {
                if visual_row == row {
                    return (byte_start + chunk_index * columns + column.min(chunk.len()))
                        .min(text.len());
                }
                visual_row += 1;
            }
        }
        byte_start += line.len() + 1;
    }
    text.len()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
    cursor: usize,
    preferred_column: Option<usize>,
}

impl<const N: usize> TextBuffer<N> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
            cursor: 0,
            preferred_column: None,
        }
    }

    pub fn insert(&mut self, character: char) -> Result<(), TextBufferError> {
        let byte = match character {
            '\n' => b'\n',
            character if character.is_ascii_graphic() || character == ' ' => character as u8,
            _ => return Err(TextBufferError::UnsupportedCharacter),
        };
        if self.len == N {
            return Err(TextBufferError::TooLong);
        }

        self.bytes
            .copy_within(self.cursor..self.len, self.cursor + 1);
        self.bytes[self.cursor] = byte;
        self.len += 1;
        self.cursor += 1;
        self.preferred_column = None;
        Ok(())
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.bytes
            .copy_within(self.cursor..self.len, self.cursor - 1);
        self.cursor -= 1;
        self.len -= 1;
        self.bytes[self.len] = 0;
        self.preferred_column = None;
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor == self.len {
            return false;
        }
        self.bytes
            .copy_within(self.cursor + 1..self.len, self.cursor);
        self.len -= 1;
        self.bytes[self.len] = 0;
        self.preferred_column = None;
        true
    }

    pub fn move_left(&mut self) -> bool {
        self.preferred_column = None;
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    pub fn move_right(&mut self) -> bool {
        self.preferred_column = None;
        if self.cursor == self.len {
            return false;
        }
        self.cursor += 1;
        true
    }

    pub fn move_home(&mut self) -> bool {
        self.preferred_column = None;
        let line_start = self.line_start(self.cursor);
        let changed = self.cursor != line_start;
        self.cursor = line_start;
        changed
    }

    pub fn move_end(&mut self) -> bool {
        self.preferred_column = None;
        let line_end = self.line_end(self.cursor);
        let changed = self.cursor != line_end;
        self.cursor = line_end;
        changed
    }

    pub fn move_up(&mut self) -> bool {
        let current_line_start = self.line_start(self.cursor);
        if current_line_start == 0 {
            return false;
        }

        let goal_column = *self
            .preferred_column
            .get_or_insert(self.cursor - current_line_start);
        let previous_line_end = current_line_start - 1;
        let previous_line_start = self.line_start(previous_line_end);
        self.cursor =
            previous_line_start + goal_column.min(previous_line_end - previous_line_start);
        true
    }

    pub fn move_down(&mut self) -> bool {
        let current_line_end = self.line_end(self.cursor);
        if current_line_end == self.len {
            return false;
        }

        let goal_column = *self
            .preferred_column
            .get_or_insert(self.cursor - self.line_start(self.cursor));
        let next_line_start = current_line_end + 1;
        let next_line_end = self.line_end(next_line_start);
        self.cursor = next_line_start + goal_column.min(next_line_end - next_line_start);
        true
    }

    pub fn clear(&mut self) {
        self.bytes[..self.len].fill(0);
        self.len = 0;
        self.cursor = 0;
        self.preferred_column = None;
    }

    pub fn replace(&mut self, contents: &[u8]) -> Result<(), TextBufferError> {
        if contents.len() > N {
            return Err(TextBufferError::TooLong);
        }
        if contents
            .iter()
            .any(|byte| !(*byte == b'\n' || (0x20..=0x7e).contains(byte)))
        {
            return Err(TextBufferError::UnsupportedCharacter);
        }

        self.clear();
        self.bytes[..contents.len()].copy_from_slice(contents);
        self.len = contents.len();
        self.cursor = self.len;
        self.preferred_column = None;
        Ok(())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).expect("text buffer contains only ASCII")
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, position: usize) -> bool {
        let position = position.min(self.len);
        let changed = self.cursor != position;
        self.cursor = position;
        self.preferred_column = None;
        changed
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn line_start(&self, position: usize) -> usize {
        self.bytes[..position]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1)
    }

    fn line_end(&self, position: usize) -> usize {
        position
            + self.bytes[position..self.len]
                .iter()
                .position(|byte| *byte == b'\n')
                .unwrap_or(self.len - position)
    }
}

impl<const N: usize> Default for TextBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{TextBuffer, TextBufferError};

    #[test]
    fn edits_and_replaces_ascii_documents() {
        let mut buffer = TextBuffer::<16>::new();
        buffer.insert('H').unwrap();
        buffer.insert('i').unwrap();
        buffer.insert('\n').unwrap();
        buffer.insert('!').unwrap();
        assert_eq!(buffer.as_str(), "Hi\n!");
        assert!(buffer.backspace());
        assert_eq!(buffer.as_str(), "Hi\n");
        buffer.replace(b"Saved text").unwrap();
        assert_eq!(buffer.as_str(), "Saved text");
    }

    #[test]
    fn rejects_overflow_and_non_ascii_text() {
        let mut buffer = TextBuffer::<3>::new();
        assert_eq!(buffer.replace(b"four"), Err(TextBufferError::TooLong));
        assert_eq!(
            buffer.replace("é".as_bytes()),
            Err(TextBufferError::UnsupportedCharacter)
        );
        buffer.insert('a').unwrap();
        buffer.insert('b').unwrap();
        buffer.insert('c').unwrap();
        assert_eq!(buffer.insert('d'), Err(TextBufferError::TooLong));
        assert_eq!(
            buffer.insert('é'),
            Err(TextBufferError::UnsupportedCharacter)
        );
        assert_eq!(buffer.as_str(), "abc");
    }

    #[test]
    fn edits_at_caret_and_moves_between_lines() {
        let mut buffer = TextBuffer::<16>::new();
        buffer.replace(b"abc\nxy\n1234").unwrap();
        assert!(buffer.move_up());
        assert_eq!(buffer.cursor(), 6);
        assert!(buffer.move_up());
        assert_eq!(buffer.cursor(), 3);
        buffer.insert('!').unwrap();
        assert_eq!(buffer.as_str(), "abc!\nxy\n1234");
        assert!(buffer.backspace());
        assert_eq!(buffer.as_str(), "abc\nxy\n1234");
        assert!(buffer.move_home());
        assert_eq!(buffer.cursor(), 0);
        assert!(!buffer.backspace());
    }

    #[test]
    fn delete_removes_the_character_at_the_caret() {
        let mut buffer = TextBuffer::<8>::new();
        buffer.replace(b"abc").unwrap();
        assert!(buffer.move_left());
        assert!(buffer.delete());
        assert_eq!(buffer.as_str(), "ab");
        assert_eq!(buffer.cursor(), 2);
        assert!(!buffer.delete());
    }

    #[test]
    fn maps_visual_carets_across_wrapped_lines() {
        let text = "abcdef\nx\n";
        assert_eq!(super::visual_cursor(text, 6, 3), (2, 0));
        assert_eq!(super::cursor_at_visual(text, 1, 2, 3), 5);
        assert_eq!(super::cursor_at_visual(text, 3, 0, 3), 9);
        assert_eq!(super::cursor_at_visual(text, 99, 0, 3), text.len());
    }
}
