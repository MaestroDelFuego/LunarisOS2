use crate::explorer::ExplorerDialog;
use crate::notepad::{FileDialogMode, FileDialogView};
use font8x8::{UnicodeFonts, BASIC_FONTS};
use myos_filesystem::{DirectoryEntry, EntryKind};
use myos_gui_core::{
    cursor_at_visual, visual_cursor, Button, ButtonState, Color, Point, Rect, StartMenuState,
    Window, WindowManager, WindowState,
};
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};

const TASKBAR_HEIGHT: usize = 40;
pub const START_MENU_ITEM_COUNT: usize = 4;
const START_MENU_HEIGHT: usize = 218;
const CALCULATOR_KEYS: [[char; 4]; 5] = [
    ['7', '8', '9', '/'],
    ['4', '5', '6', '*'],
    ['1', '2', '3', '-'],
    ['C', '0', '.', '+'],
    ['\u{8}', '=', '\0', '\0'],
];

pub fn taskbar_start_bounds(screen_height: usize) -> Rect {
    Rect::new(6, screen_height.saturating_sub(36) as i32, 88, 32)
}

pub fn taskbar_window_bounds(index: usize, screen_height: usize) -> Rect {
    Rect::new(
        102 + (index as i32 * 166),
        screen_height.saturating_sub(36) as i32,
        160,
        32,
    )
}

pub fn start_menu_entry_bounds(index: usize, screen_height: usize) -> Rect {
    let panel_top = screen_height.saturating_sub(TASKBAR_HEIGHT + START_MENU_HEIGHT) as i32;
    Rect::new(14, panel_top + 42 + index as i32 * 38, 234, 34)
}

pub fn calculator_key_bounds(bounds: Rect, key: char) -> Option<Rect> {
    let (row, column) = CALCULATOR_KEYS.iter().enumerate().find_map(|(row, keys)| {
        keys.iter()
            .position(|candidate| *candidate == key)
            .map(|column| (row, column))
    })?;
    let width = if key == '=' { 148 } else { 70 };
    Some(Rect::new(
        bounds.x + 16 + column as i32 * 78,
        bounds.y + 112 + row as i32 * 50,
        width,
        42,
    ))
}

pub fn calculator_key_at(bounds: Rect, point: Point) -> Option<char> {
    CALCULATOR_KEYS
        .iter()
        .flatten()
        .copied()
        .filter(|key| *key != '\0')
        .find(|key| {
            calculator_key_bounds(bounds, *key).is_some_and(|button| button.contains(point))
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

pub fn window_control_bounds(bounds: Rect, control: WindowControl) -> Rect {
    let offset = match control {
        WindowControl::Close => 4,
        WindowControl::Maximize => 25,
        WindowControl::Minimize => 46,
    };
    Rect::new(
        bounds.x + bounds.width as i32 - offset - 18,
        bounds.y + 5,
        18,
        18,
    )
}

pub fn window_control_at(bounds: Rect, point: Point) -> Option<WindowControl> {
    [
        WindowControl::Close,
        WindowControl::Maximize,
        WindowControl::Minimize,
    ]
    .into_iter()
    .find(|control| window_control_bounds(bounds, *control).contains(point))
}

pub fn notepad_open_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 10, bounds.y + 35, 58, 18)
}

pub fn notepad_save_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 73, bounds.y + 35, 58, 18)
}

pub fn notepad_save_as_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 136, bounds.y + 35, 72, 18)
}

pub fn file_dialog_bounds(bounds: Rect) -> Rect {
    Rect::new(
        bounds.x + 40,
        bounds.y + 67,
        bounds.width.saturating_sub(80),
        bounds.height.saturating_sub(132),
    )
}

pub fn file_dialog_row_bounds(bounds: Rect, row: usize) -> Rect {
    let dialog = file_dialog_bounds(bounds);
    Rect::new(
        dialog.x + 14,
        dialog.y + 94 + row as i32 * 24,
        dialog.width - 28,
        22,
    )
}

pub fn file_dialog_filename_bounds(bounds: Rect) -> Rect {
    let dialog = file_dialog_bounds(bounds);
    Rect::new(dialog.x + 14, dialog.y + 58, dialog.width - 28, 24)
}

pub fn file_dialog_accept_bounds(bounds: Rect) -> Rect {
    let dialog = file_dialog_bounds(bounds);
    Rect::new(
        dialog.x + dialog.width as i32 - 166,
        dialog.y + dialog.height as i32 - 36,
        76,
        24,
    )
}

pub fn file_dialog_cancel_bounds(bounds: Rect) -> Rect {
    let dialog = file_dialog_bounds(bounds);
    Rect::new(
        dialog.x + dialog.width as i32 - 82,
        dialog.y + dialog.height as i32 - 36,
        68,
        24,
    )
}

pub fn explorer_row_bounds(bounds: Rect, row: usize) -> Rect {
    Rect::new(
        bounds.x + 12,
        bounds.y + 106 + row as i32 * 26,
        bounds.width.saturating_sub(24),
        24,
    )
}

pub fn explorer_parent_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 12, bounds.y + 40, 54, 20)
}

pub fn explorer_create_directory_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 242, bounds.y + 40, 64, 20)
}

pub fn explorer_create_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 314, bounds.y + 40, 50, 20)
}

pub fn explorer_rename_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 368, bounds.y + 40, 56, 20)
}

pub fn explorer_delete_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 428, bounds.y + 40, 56, 20)
}

pub fn explorer_copy_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 488, bounds.y + 40, 46, 20)
}

pub fn explorer_paste_bounds(bounds: Rect) -> Rect {
    Rect::new(bounds.x + 538, bounds.y + 40, 50, 20)
}

pub fn explorer_context_menu_bounds(
    position: (i32, i32),
    screen_width: usize,
    screen_height: usize,
) -> Rect {
    const MENU_WIDTH: u32 = 128;
    const MENU_HEIGHT: u32 = 158;
    Rect::new(
        position
            .0
            .clamp(0, screen_width.saturating_sub(MENU_WIDTH as usize) as i32),
        position
            .1
            .clamp(0, screen_height.saturating_sub(MENU_HEIGHT as usize) as i32),
        MENU_WIDTH,
        MENU_HEIGHT,
    )
}

pub fn explorer_context_menu_item_bounds(menu: Rect, item: usize) -> Rect {
    Rect::new(
        menu.x + 2,
        menu.y + 2 + item as i32 * 22,
        menu.width - 4,
        22,
    )
}

pub fn explorer_context_menu_item_at(
    position: (i32, i32),
    point: Point,
    screen_width: usize,
    screen_height: usize,
) -> Option<usize> {
    let menu = explorer_context_menu_bounds(position, screen_width, screen_height);
    (0..7).find(|item| explorer_context_menu_item_bounds(menu, *item).contains(point))
}

pub fn explorer_dialog_bounds(bounds: Rect) -> Rect {
    Rect::new(
        bounds.x + 100,
        bounds.y + 120,
        bounds.width.saturating_sub(200),
        170,
    )
}

pub fn explorer_dialog_accept_bounds(bounds: Rect) -> Rect {
    let dialog = explorer_dialog_bounds(bounds);
    Rect::new(dialog.x + dialog.width as i32 - 158, dialog.y + 132, 72, 24)
}

pub fn explorer_dialog_cancel_bounds(bounds: Rect) -> Rect {
    let dialog = explorer_dialog_bounds(bounds);
    Rect::new(dialog.x + dialog.width as i32 - 78, dialog.y + 132, 64, 24)
}

pub fn notepad_caret_at(bounds: Rect, text: &str, cursor: usize, point: Point) -> Option<usize> {
    let body = Rect::new(
        bounds.x + 10,
        bounds.y + 56,
        bounds.width.saturating_sub(20),
        bounds.height.saturating_sub(78),
    );
    if !body.contains(point) || body.width < 8 || body.height < 12 {
        return None;
    }

    let columns = (body.width as usize / 8).max(1);
    let rows = body.height as usize / 12;
    let target_column = ((point.x - body.x - 4 + 4).max(0) as usize / 8).min(columns);
    let target_row = ((point.y - body.y - 2).max(0) as usize / 12).min(rows - 1);
    let cursor_visual = visual_cursor(text, cursor.min(text.len()), columns);
    let first_row = cursor_visual.0.saturating_sub(rows.saturating_sub(1));
    let target_row = first_row + target_row;
    Some(cursor_at_visual(text, target_row, target_column, columns))
}

pub struct GopCanvas<'gop> {
    gop: &'gop mut GraphicsOutput,
    width: usize,
    height: usize,
    clip: Option<Rect>,
}

impl<'gop> GopCanvas<'gop> {
    pub const fn new(gop: &'gop mut GraphicsOutput, width: usize, height: usize) -> Self {
        Self {
            gop,
            width,
            height,
            clip: None,
        }
    }

    pub fn draw_wallpaper(&mut self) -> uefi::Result {
        crate::wallpaper::draw(self.gop, self.width, self.height, self.clip)
    }

    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.clip = clip;
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) -> uefi::Result {
        let mut left = i64::from(rect.x).max(0);
        let mut top = i64::from(rect.y).max(0);
        let mut right = (i64::from(rect.x) + i64::from(rect.width)).min(self.width as i64);
        let mut bottom = (i64::from(rect.y) + i64::from(rect.height)).min(self.height as i64);

        if let Some(clip) = self.clip {
            left = left.max(i64::from(clip.x));
            top = top.max(i64::from(clip.y));
            right = right.min(i64::from(clip.x) + i64::from(clip.width));
            bottom = bottom.min(i64::from(clip.y) + i64::from(clip.height));
        }

        if right <= left || bottom <= top {
            return Ok(());
        }

        self.gop.blt(BltOp::VideoFill {
            color: BltPixel::new(color.red, color.green, color.blue),
            dest: (left as usize, top as usize),
            dims: ((right - left) as usize, (bottom - top) as usize),
        })
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, color: Color) -> uefi::Result {
        let mut cursor_x = x;

        for character in text.chars() {
            if let Some(glyph) = BASIC_FONTS.get(character) {
                for (row_index, row_bits) in glyph.iter().copied().enumerate() {
                    let mut column = 0;
                    while column < 8 {
                        if row_bits & (1 << column) == 0 {
                            column += 1;
                            continue;
                        }

                        let run_start = column;
                        while column < 8 && row_bits & (1 << column) != 0 {
                            column += 1;
                        }

                        self.fill_rect(
                            Rect::new(
                                cursor_x + run_start,
                                y + row_index as i32,
                                (column - run_start) as u32,
                                1,
                            ),
                            color,
                        )?;
                    }
                }
            }

            cursor_x += 8;
        }

        Ok(())
    }

    pub fn draw_button(&mut self, button: &Button) -> uefi::Result {
        let bounds = button.bounds();
        let face = match button.state() {
            ButtonState::Normal => Color::new(222, 231, 241),
            ButtonState::Hovered => Color::new(238, 245, 252),
            ButtonState::Pressed => Color::new(194, 210, 226),
            ButtonState::Disabled => Color::new(207, 211, 216),
        };
        let light = Color::new(255, 255, 255);
        let dark = Color::new(73, 96, 119);
        self.fill_rect(bounds, face)?;

        let (upper, lower) = if button.state() == ButtonState::Pressed {
            (dark, light)
        } else {
            (light, dark)
        };
        self.fill_rect(Rect::new(bounds.x, bounds.y, bounds.width, 1), upper)?;
        self.fill_rect(Rect::new(bounds.x, bounds.y, 1, bounds.height), upper)?;
        self.fill_rect(
            Rect::new(
                bounds.x,
                bounds.y + bounds.height as i32 - 1,
                bounds.width,
                1,
            ),
            lower,
        )?;
        self.fill_rect(
            Rect::new(
                bounds.x + bounds.width as i32 - 1,
                bounds.y,
                1,
                bounds.height,
            ),
            lower,
        )?;

        let text_width = button.label().len().saturating_mul(8);
        let text_x = bounds.x + bounds.width.saturating_sub(text_width as u32) as i32 / 2;
        let text_y = bounds.y + bounds.height.saturating_sub(8) as i32 / 2;
        let text_color = if button.state() == ButtonState::Disabled {
            Color::new(112, 118, 124)
        } else {
            Color::new(24, 38, 52)
        };
        self.draw_text(text_x, text_y, button.label(), text_color)
    }

    pub fn draw_desktop_icon(&mut self, icon: &Button) -> uefi::Result {
        let bounds = icon.bounds();
        if icon.state() == ButtonState::Hovered || icon.state() == ButtonState::Pressed {
            self.fill_rect(bounds, Color::new(34, 91, 151))?;
        }

        let icon_width = 44;
        let icon_height = 36;
        let icon_x = bounds.x + (bounds.width as i32 - icon_width) / 2;
        let icon_y = bounds.y + 6;
        self.fill_rect(
            Rect::new(
                icon_x - 1,
                icon_y - 1,
                (icon_width + 2) as u32,
                (icon_height + 2) as u32,
            ),
            Color::new(28, 42, 57),
        )?;
        self.fill_rect(
            Rect::new(icon_x, icon_y, icon_width as u32, icon_height as u32),
            Color::new(239, 243, 247),
        )?;
        self.fill_rect(
            Rect::new(icon_x, icon_y, icon_width as u32, 10),
            Color::new(35, 103, 180),
        )?;
        self.fill_rect(
            Rect::new(icon_x + 5, icon_y + 17, 28, 2),
            Color::new(120, 137, 153),
        )?;
        self.fill_rect(
            Rect::new(icon_x + 5, icon_y + 23, 21, 2),
            Color::new(120, 137, 153),
        )?;
        self.fill_rect(
            Rect::new(icon_x + 5, icon_y + 29, 25, 2),
            Color::new(120, 137, 153),
        )?;

        let label_width = icon.label().len().saturating_mul(8) as i32;
        let label_x = bounds.x + (bounds.width as i32 - label_width) / 2;
        let label_y = bounds.y + 62;
        self.draw_text(
            label_x + 1,
            label_y + 1,
            icon.label(),
            Color::new(18, 29, 39),
        )?;
        self.draw_text(label_x, label_y, icon.label(), Color::new(255, 255, 255))
    }

    pub fn draw_taskbar<const N: usize>(
        &mut self,
        manager: &WindowManager<N>,
        start_menu: &StartMenuState,
        clock: &str,
    ) -> uefi::Result {
        let taskbar_y = self.height.saturating_sub(TASKBAR_HEIGHT);
        self.fill_rect(
            Rect::new(
                0,
                taskbar_y as i32,
                self.width as u32,
                TASKBAR_HEIGHT as u32,
            ),
            Color::new(25, 83, 151),
        )?;
        self.fill_rect(
            Rect::new(0, taskbar_y as i32, self.width as u32, 2),
            Color::new(122, 176, 222),
        )?;

        let start = taskbar_start_bounds(self.height);
        let start_face = if start_menu.is_open() {
            Color::new(81, 176, 56)
        } else {
            Color::new(54, 143, 45)
        };
        self.fill_rect(start, Color::new(28, 87, 31))?;
        self.fill_rect(
            Rect::new(start.x + 1, start.y + 1, start.width - 2, start.height - 2),
            start_face,
        )?;
        self.draw_text(
            start.x + 24,
            start.y + 12,
            "Start",
            Color::new(255, 255, 255),
        )?;

        let task_buttons_end = self.width.saturating_sub(70);
        for (index, window) in manager.windows_back_to_front().enumerate() {
            let button = taskbar_window_bounds(index, self.height);
            if button.x as usize + button.width as usize > task_buttons_end {
                break;
            }

            let active = manager.active_id() == Some(window.id());
            let face = if active {
                Color::new(69, 125, 183)
            } else {
                Color::new(37, 94, 154)
            };
            self.fill_rect(button, Color::new(19, 57, 103))?;
            self.fill_rect(
                Rect::new(
                    button.x + 1,
                    button.y + 1,
                    button.width - 2,
                    button.height - 2,
                ),
                face,
            )?;
            self.draw_text(
                button.x + 8,
                button.y + 12,
                window.title(),
                Color::new(255, 255, 255),
            )?;
        }

        self.draw_text(
            self.width.saturating_sub(48) as i32,
            taskbar_y as i32 + 15,
            clock,
            Color::new(255, 255, 255),
        )
    }

    pub fn draw_start_menu(&mut self, menu: &StartMenuState) -> uefi::Result {
        if !menu.is_open() {
            return Ok(());
        }

        let panel_y = self
            .height
            .saturating_sub(TASKBAR_HEIGHT + START_MENU_HEIGHT) as i32;
        let panel = Rect::new(6, panel_y, 250, START_MENU_HEIGHT as u32);
        self.fill_rect(panel, Color::new(31, 54, 80))?;
        self.fill_rect(
            Rect::new(panel.x + 1, panel.y + 1, panel.width - 2, panel.height - 2),
            Color::new(239, 242, 246),
        )?;
        self.fill_rect(
            Rect::new(panel.x + 1, panel.y + 1, panel.width - 2, 30),
            Color::new(31, 93, 160),
        )?;
        self.draw_text(
            panel.x + 10,
            panel.y + 12,
            "MYOS",
            Color::new(255, 255, 255),
        )?;

        for (index, label) in ["Window Demo", "Notepad", "File Explorer", "Calculator"]
            .iter()
            .enumerate()
        {
            let entry = start_menu_entry_bounds(index, self.height);
            let selected = menu.selected_item() == index;
            self.fill_rect(
                entry,
                if selected {
                    Color::new(54, 117, 181)
                } else {
                    Color::new(239, 242, 246)
                },
            )?;
            self.fill_rect(
                Rect::new(entry.x + 8, entry.y + 8, 22, 18),
                Color::new(31, 93, 160),
            )?;
            self.fill_rect(
                Rect::new(entry.x + 10, entry.y + 10, 18, 4),
                Color::new(196, 221, 242),
            )?;
            self.draw_text(
                entry.x + 42,
                entry.y + 13,
                label,
                if selected {
                    Color::new(255, 255, 255)
                } else {
                    Color::new(32, 44, 57)
                },
            )?;
        }

        Ok(())
    }

    pub fn draw_window(&mut self, window: &Window, focused: bool) -> uefi::Result {
        if window.state() == WindowState::Minimized {
            return Ok(());
        }

        let bounds = window.bounds();
        if bounds.width < 4 || bounds.height < 4 {
            return Ok(());
        }

        let border = Color::new(35, 55, 78);
        let client = Color::new(240, 242, 245);
        let title = if focused {
            Color::new(28, 94, 176)
        } else {
            Color::new(92, 116, 142)
        };
        let inner_bounds = Rect::new(
            bounds.x + 1,
            bounds.y + 1,
            bounds.width - 2,
            bounds.height - 2,
        );
        self.fill_rect(bounds, border)?;
        self.fill_rect(inner_bounds, client)?;

        let title_height = inner_bounds.height.min(28);
        self.fill_rect(
            Rect::new(
                inner_bounds.x,
                inner_bounds.y,
                inner_bounds.width,
                title_height,
            ),
            title,
        )?;
        self.draw_text(
            inner_bounds.x + 9,
            inner_bounds.y + 9,
            window.title(),
            Color::new(255, 255, 255),
        )?;

        for control in [
            WindowControl::Minimize,
            WindowControl::Maximize,
            WindowControl::Close,
        ] {
            let control_bounds = window_control_bounds(bounds, control);
            let face = if control == WindowControl::Close {
                Color::new(202, 66, 48)
            } else {
                Color::new(190, 211, 233)
            };
            self.fill_rect(control_bounds, face)?;
            let label = match control {
                WindowControl::Minimize => "_",
                WindowControl::Maximize => "[]",
                WindowControl::Close => "X",
            };
            self.draw_text(
                control_bounds.x
                    + if control == WindowControl::Maximize {
                        3
                    } else {
                        5
                    },
                control_bounds.y + 5,
                label,
                if control == WindowControl::Close {
                    Color::new(255, 255, 255)
                } else {
                    Color::new(30, 47, 63)
                },
            )?;
        }

        Ok(())
    }

    pub fn draw_notepad(
        &mut self,
        window: &Window,
        text: &str,
        cursor: usize,
        status: &str,
        filename: &str,
        dialog: Option<FileDialogView<'_>>,
        focused: bool,
    ) -> uefi::Result {
        let bounds = window.bounds();
        let left = bounds.x + 10;
        let right = bounds.x + bounds.width as i32 - 10;
        let top = bounds.y + 38;
        let bottom = bounds.y + bounds.height as i32 - 22;
        let body = Rect::new(
            left,
            top + 18,
            right.saturating_sub(left) as u32,
            bottom.saturating_sub(top + 18) as u32,
        );

        for (button, label) in [
            (notepad_open_bounds(bounds), "Open"),
            (notepad_save_bounds(bounds), "Save"),
            (notepad_save_as_bounds(bounds), "Save As"),
        ] {
            self.fill_rect(button, Color::new(90, 111, 132))?;
            self.fill_rect(
                Rect::new(
                    button.x + 1,
                    button.y + 1,
                    button.width - 2,
                    button.height - 2,
                ),
                Color::new(225, 233, 241),
            )?;
            self.draw_text(
                button.x + if label == "Save As" { 7 } else { 17 },
                button.y + 5,
                label,
                Color::new(30, 43, 56),
            )?;
        }
        self.draw_text(
            left + 216,
            top + 5,
            "F5 Open F6 Save F7 Save As",
            Color::new(35, 47, 61),
        )?;
        self.draw_text(left + 432, top + 5, filename, Color::new(35, 47, 61))?;
        self.fill_rect(body, Color::new(255, 255, 255))?;
        let columns = (body.width as usize / 8).max(1);
        let rows = body.height as usize / 12;
        let bytes = text.as_bytes();
        let mut cursor_row: usize = 0;
        let mut cursor_column: usize = 0;
        for byte in bytes.iter().take(cursor.min(bytes.len())) {
            if *byte == b'\n' {
                cursor_row += 1;
                cursor_column = 0;
            } else {
                cursor_column += 1;
                if cursor_column == columns {
                    cursor_row += 1;
                    cursor_column = 0;
                }
            }
        }
        let first_row = cursor_row.saturating_sub(rows.saturating_sub(1));
        let mut row = 0;

        for line in text.split('\n') {
            if row >= first_row.saturating_add(rows) {
                break;
            }
            if line.is_empty() {
                row += 1;
                continue;
            }
            for chunk in line.as_bytes().chunks(columns) {
                if row >= first_row.saturating_add(rows) {
                    break;
                }
                if row >= first_row {
                    let line = core::str::from_utf8(chunk).unwrap_or("");
                    self.draw_text(
                        body.x + 4,
                        body.y + (row - first_row) as i32 * 12 + 2,
                        line,
                        Color::new(24, 32, 42),
                    )?;
                }
                row += 1;
            }
        }

        if focused && cursor_row >= first_row && cursor_row < first_row + rows {
            self.fill_rect(
                Rect::new(
                    body.x + 4 + cursor_column as i32 * 8,
                    body.y + (cursor_row - first_row) as i32 * 12 + 2,
                    1,
                    9,
                ),
                Color::new(20, 27, 36),
            )?;
        }

        self.fill_rect(
            Rect::new(
                left,
                bounds.y + bounds.height as i32 - 18,
                right.saturating_sub(left) as u32,
                14,
            ),
            Color::new(220, 228, 237),
        )?;
        self.draw_text(
            left + 4,
            bounds.y + bounds.height as i32 - 15,
            status,
            Color::new(35, 47, 61),
        )?;
        if let Some(dialog) = dialog {
            self.draw_file_dialog(bounds, dialog)?;
        }
        Ok(())
    }

    pub fn draw_explorer(
        &mut self,
        window: &Window,
        entries: &[DirectoryEntry],
        selected: Option<usize>,
        visible_start: usize,
        location: &str,
        status: &str,
        dialog: Option<ExplorerDialog>,
        edit_name: &str,
        context_menu: Option<(i32, i32)>,
    ) -> uefi::Result {
        let bounds = window.bounds();
        let list = Rect::new(
            bounds.x + 10,
            bounds.y + 80,
            bounds.width.saturating_sub(20),
            bounds.height.saturating_sub(102),
        );
        for (button, label) in [
            (explorer_parent_bounds(bounds), "Up"),
            (explorer_create_directory_bounds(bounds), "Folder"),
            (explorer_create_bounds(bounds), "New"),
            (explorer_rename_bounds(bounds), "Rename"),
            (explorer_delete_bounds(bounds), "Delete"),
            (explorer_copy_bounds(bounds), "Copy"),
            (explorer_paste_bounds(bounds), "Paste"),
        ] {
            self.fill_rect(button, Color::new(90, 111, 132))?;
            self.fill_rect(
                Rect::new(
                    button.x + 1,
                    button.y + 1,
                    button.width - 2,
                    button.height - 2,
                ),
                Color::new(225, 233, 241),
            )?;
            self.draw_text(button.x + 5, button.y + 7, label, Color::new(30, 43, 56))?;
        }
        self.draw_text(
            bounds.x + 14,
            bounds.y + 66,
            location,
            Color::new(35, 47, 61),
        )?;
        self.fill_rect(list, Color::new(255, 255, 255))?;
        self.fill_rect(
            Rect::new(list.x, list.y, list.width, 24),
            Color::new(220, 228, 237),
        )?;
        self.draw_text(list.x + 8, list.y + 8, "Name", Color::new(35, 47, 61))?;
        self.draw_text(
            list.x + list.width as i32 - 100,
            list.y + 8,
            "Size (bytes)",
            Color::new(35, 47, 61),
        )?;

        if entries.is_empty() {
            self.draw_text(list.x + 12, list.y + 42, status, Color::new(80, 90, 100))?;
        } else {
            let rows = ((list.height.saturating_sub(26)) / 26) as usize;
            for (row_index, entry) in entries.iter().skip(visible_start).take(rows).enumerate() {
                let index = visible_start + row_index;
                let row = explorer_row_bounds(bounds, row_index);
                let is_selected = selected == Some(index);
                self.fill_rect(
                    row,
                    if is_selected {
                        Color::new(54, 117, 181)
                    } else {
                        Color::new(255, 255, 255)
                    },
                )?;
                let foreground = if is_selected {
                    Color::new(255, 255, 255)
                } else {
                    Color::new(35, 47, 61)
                };
                let name_x = if entry.kind == EntryKind::Directory {
                    self.fill_rect(
                        Rect::new(row.x + 8, row.y + 7, 10, 10),
                        Color::new(225, 181, 67),
                    )?;
                    row.x + 24
                } else {
                    row.x + 8
                };
                self.draw_text(name_x, row.y + 8, entry.name(), foreground)?;
                if entry.kind == EntryKind::Directory {
                    self.draw_text(
                        row.x + row.width as i32 - 100,
                        row.y + 8,
                        "<DIR>",
                        foreground,
                    )?;
                    continue;
                }
                let mut digits = [0u8; 10];
                let mut value = entry.size;
                let mut length = 0;
                loop {
                    digits[length] = b'0' + (value % 10) as u8;
                    length += 1;
                    value /= 10;
                    if value == 0 {
                        break;
                    }
                }
                digits[..length].reverse();
                let size = core::str::from_utf8(&digits[..length]).unwrap_or("0");
                self.draw_text(row.x + row.width as i32 - 100, row.y + 8, size, foreground)?;
            }
        }

        self.fill_rect(
            Rect::new(
                bounds.x + 10,
                bounds.y + bounds.height as i32 - 22,
                bounds.width.saturating_sub(20),
                16,
            ),
            Color::new(220, 228, 237),
        )?;
        self.draw_text(
            bounds.x + 14,
            bounds.y + bounds.height as i32 - 19,
            status,
            Color::new(35, 47, 61),
        )?;
        if let Some(dialog) = dialog {
            let selected_entry = selected.and_then(|index| entries.get(index)).copied();
            self.draw_explorer_dialog(bounds, dialog, edit_name, selected_entry)?;
        } else if let Some(position) = context_menu {
            self.draw_explorer_context_menu(position)?;
        }
        Ok(())
    }

    pub fn draw_calculator(
        &mut self,
        window: &Window,
        calculator: &myos_gui_core::Calculator,
    ) -> uefi::Result {
        let bounds = window.bounds();
        let display = Rect::new(bounds.x + 16, bounds.y + 48, bounds.width - 32, 48);
        self.fill_rect(display, Color::new(25, 41, 56))?;
        self.fill_rect(
            Rect::new(
                display.x + 2,
                display.y + 2,
                display.width - 4,
                display.height - 4,
            ),
            Color::new(250, 252, 254),
        )?;
        self.draw_text(
            display.x + 10,
            display.y + 19,
            calculator.display(),
            Color::new(35, 47, 61),
        )?;
        if let Some(operation) = calculator.pending_operation() {
            let key = [operation as u8];
            let label = core::str::from_utf8(&key).unwrap_or("");
            self.draw_text(
                display.x + display.width as i32 - 20,
                display.y + 19,
                label,
                Color::new(35, 47, 61),
            )?;
        }

        for key in CALCULATOR_KEYS
            .iter()
            .flatten()
            .copied()
            .filter(|key| *key != '\0')
        {
            let button = calculator_key_bounds(bounds, key).unwrap();
            self.fill_rect(button, Color::new(60, 79, 98))?;
            self.fill_rect(
                Rect::new(
                    button.x + 1,
                    button.y + 1,
                    button.width - 2,
                    button.height - 2,
                ),
                if matches!(key, '+' | '-' | '*' | '/' | '=') {
                    Color::new(199, 218, 234)
                } else {
                    Color::new(232, 237, 242)
                },
            )?;
            let label = match key {
                '\u{8}' => "BS",
                _ => {
                    let text = [key as u8];
                    let text = core::str::from_utf8(&text).unwrap_or("");
                    self.draw_text(
                        button.x + button.width as i32 / 2 - 4,
                        button.y + 17,
                        text,
                        Color::new(30, 43, 56),
                    )?;
                    continue;
                }
            };
            self.draw_text(
                button.x + button.width as i32 / 2 - 8,
                button.y + 17,
                label,
                Color::new(30, 43, 56),
            )?;
        }
        Ok(())
    }

    fn draw_explorer_context_menu(&mut self, position: (i32, i32)) -> uefi::Result {
        let menu = explorer_context_menu_bounds(position, self.width, self.height);
        self.fill_rect(menu, Color::new(37, 54, 72))?;
        self.fill_rect(
            Rect::new(menu.x + 1, menu.y + 1, menu.width - 2, menu.height - 2),
            Color::new(248, 249, 251),
        )?;
        for (index, label) in [
            "New file",
            "Open",
            "Rename",
            "Delete",
            "Copy",
            "Paste",
            "Properties",
        ]
        .iter()
        .enumerate()
        {
            let item = explorer_context_menu_item_bounds(menu, index);
            self.draw_text(item.x + 8, item.y + 7, label, Color::new(30, 43, 56))?;
            if index == 0 {
                self.fill_rect(
                    Rect::new(
                        item.x + 4,
                        item.y + item.height as i32 - 1,
                        item.width - 8,
                        1,
                    ),
                    Color::new(205, 212, 220),
                )?;
            }
        }
        Ok(())
    }

    fn draw_explorer_dialog(
        &mut self,
        bounds: Rect,
        mode: ExplorerDialog,
        edit_name: &str,
        selected_entry: Option<DirectoryEntry>,
    ) -> uefi::Result {
        let dialog = explorer_dialog_bounds(bounds);
        self.fill_rect(dialog, Color::new(37, 54, 72))?;
        self.fill_rect(
            Rect::new(
                dialog.x + 2,
                dialog.y + 2,
                dialog.width - 4,
                dialog.height - 4,
            ),
            Color::new(245, 247, 250),
        )?;
        let (title, prompt) = match mode {
            ExplorerDialog::Create => ("Create file", "New filename:"),
            ExplorerDialog::CreateDirectory => ("Create folder", "New folder name:"),
            ExplorerDialog::Rename => ("Rename item", "New name:"),
            ExplorerDialog::Paste => ("Paste file", "Destination filename:"),
            ExplorerDialog::ConfirmDelete => ("Delete item", "Delete this item permanently?"),
            ExplorerDialog::Properties => ("Item properties", "Item information:"),
        };
        self.fill_rect(
            Rect::new(dialog.x + 2, dialog.y + 2, dialog.width - 4, 28),
            Color::new(31, 93, 160),
        )?;
        self.draw_text(
            dialog.x + 12,
            dialog.y + 11,
            title,
            Color::new(255, 255, 255),
        )?;
        self.draw_text(dialog.x + 14, dialog.y + 54, prompt, Color::new(35, 47, 61))?;
        if mode == ExplorerDialog::Properties {
            if let Some(entry) = selected_entry {
                self.draw_text(
                    dialog.x + 14,
                    dialog.y + 76,
                    entry.name(),
                    Color::new(35, 47, 61),
                )?;
                let mut digits = [0u8; 10];
                let mut value = entry.size;
                let mut length = 0;
                loop {
                    digits[length] = b'0' + (value % 10) as u8;
                    length += 1;
                    value /= 10;
                    if value == 0 {
                        break;
                    }
                }
                digits[..length].reverse();
                let size = core::str::from_utf8(&digits[..length]).unwrap_or("0");
                self.draw_text(
                    dialog.x + 14,
                    dialog.y + 96,
                    if entry.kind == EntryKind::Directory {
                        "Type: Folder"
                    } else {
                        "Type: File"
                    },
                    Color::new(35, 47, 61),
                )?;
                self.draw_text(
                    dialog.x + 14,
                    dialog.y + 116,
                    "Size (bytes):",
                    Color::new(35, 47, 61),
                )?;
                self.draw_text(dialog.x + 112, dialog.y + 116, size, Color::new(35, 47, 61))?;
            }
        } else if mode != ExplorerDialog::ConfirmDelete {
            let field = Rect::new(dialog.x + 14, dialog.y + 72, dialog.width - 28, 26);
            self.fill_rect(field, Color::new(255, 255, 255))?;
            self.draw_text(field.x + 5, field.y + 8, edit_name, Color::new(30, 43, 56))?;
        }
        let accept = explorer_dialog_accept_bounds(bounds);
        let cancel = explorer_dialog_cancel_bounds(bounds);
        let accept_label = if mode == ExplorerDialog::Properties {
            "Close"
        } else if mode == ExplorerDialog::ConfirmDelete {
            "Delete"
        } else {
            "OK"
        };
        let buttons = if mode == ExplorerDialog::Properties {
            [(accept, accept_label), (cancel, "")]
        } else {
            [(accept, accept_label), (cancel, "Cancel")]
        };
        let button_count = if mode == ExplorerDialog::Properties {
            1
        } else {
            2
        };
        for (button, label) in buttons.into_iter().take(button_count) {
            self.fill_rect(button, Color::new(90, 111, 132))?;
            self.fill_rect(
                Rect::new(
                    button.x + 1,
                    button.y + 1,
                    button.width - 2,
                    button.height - 2,
                ),
                Color::new(225, 233, 241),
            )?;
            self.draw_text(button.x + 8, button.y + 8, label, Color::new(30, 43, 56))?;
        }
        Ok(())
    }

    fn draw_file_dialog(&mut self, bounds: Rect, view: FileDialogView<'_>) -> uefi::Result {
        let dialog = file_dialog_bounds(bounds);
        self.fill_rect(dialog, Color::new(37, 54, 72))?;
        self.fill_rect(
            Rect::new(
                dialog.x + 2,
                dialog.y + 2,
                dialog.width - 4,
                dialog.height - 4,
            ),
            Color::new(245, 247, 250),
        )?;
        let title = match view.mode {
            FileDialogMode::Open => "Open file",
            FileDialogMode::SaveAs => "Save as",
            FileDialogMode::ConfirmOverwrite => "Confirm overwrite",
            FileDialogMode::ConfirmDiscard => "Confirm open",
            FileDialogMode::ConfirmClose => "Confirm close",
        };
        self.fill_rect(
            Rect::new(dialog.x + 2, dialog.y + 2, dialog.width - 4, 28),
            Color::new(31, 93, 160),
        )?;
        self.draw_text(
            dialog.x + 12,
            dialog.y + 11,
            title,
            Color::new(255, 255, 255),
        )?;

        if view.mode == FileDialogMode::SaveAs {
            let field = file_dialog_filename_bounds(bounds);
            self.fill_rect(field, Color::new(255, 255, 255))?;
            self.draw_text(
                field.x + 5,
                field.y + 7,
                view.filename,
                Color::new(30, 43, 56),
            )?;
            self.draw_text(field.x, field.y - 12, "File name:", Color::new(35, 47, 61))?;
        } else if matches!(
            view.mode,
            FileDialogMode::ConfirmOverwrite
                | FileDialogMode::ConfirmDiscard
                | FileDialogMode::ConfirmClose
        ) {
            let prompt = if view.mode == FileDialogMode::ConfirmOverwrite {
                "Overwrite the selected file?"
            } else if view.mode == FileDialogMode::ConfirmClose {
                "Discard unsaved edits and close?"
            } else {
                "Discard unsaved changes and open?"
            };
            self.draw_text(dialog.x + 14, dialog.y + 52, prompt, Color::new(35, 47, 61))?;
        }

        if matches!(view.mode, FileDialogMode::Open | FileDialogMode::SaveAs) {
            if view.files.is_empty() {
                self.draw_text(
                    dialog.x + 14,
                    dialog.y + 104,
                    "No files on this volume.",
                    Color::new(80, 90, 100),
                )?;
            }
            let visible_count = view.files.len().saturating_sub(view.visible_start).min(7);
            for row in 0..visible_count {
                let index = view.visible_start + row;
                let row_bounds = file_dialog_row_bounds(bounds, row);
                let selected = view.selected == Some(index);
                self.fill_rect(
                    row_bounds,
                    if selected {
                        Color::new(54, 117, 181)
                    } else {
                        Color::new(255, 255, 255)
                    },
                )?;
                let file = view.files[index];
                self.draw_text(
                    row_bounds.x + 5,
                    row_bounds.y + 7,
                    file.name(),
                    if selected {
                        Color::new(255, 255, 255)
                    } else {
                        Color::new(35, 47, 61)
                    },
                )?;
            }
        } else {
            self.draw_text(
                dialog.x + 14,
                dialog.y + 72,
                view.filename,
                Color::new(35, 47, 61),
            )?;
        }

        let accept = file_dialog_accept_bounds(bounds);
        let cancel = file_dialog_cancel_bounds(bounds);
        for (button, label) in [
            (
                accept,
                if view.mode == FileDialogMode::ConfirmOverwrite {
                    "Overwrite"
                } else if matches!(
                    view.mode,
                    FileDialogMode::ConfirmDiscard | FileDialogMode::ConfirmClose
                ) {
                    "Discard"
                } else if view.mode == FileDialogMode::Open {
                    "Open"
                } else {
                    "Save"
                },
            ),
            (cancel, "Cancel"),
        ] {
            self.fill_rect(button, Color::new(90, 111, 132))?;
            self.fill_rect(
                Rect::new(
                    button.x + 1,
                    button.y + 1,
                    button.width - 2,
                    button.height - 2,
                ),
                Color::new(225, 233, 241),
            )?;
            self.draw_text(button.x + 6, button.y + 8, label, Color::new(30, 43, 56))?;
        }
        Ok(())
    }
}

const CURSOR_WIDTH: usize = 16;
const CURSOR_HEIGHT: usize = 16;
const CURSOR_MASK: [&str; CURSOR_HEIGHT] = [
    "#...............",
    "##..............",
    "#W#.............",
    "#WW#............",
    "#WWW#...........",
    "#WWWW#..........",
    "#WWWWW#.........",
    "#WWWWWW#........",
    "#WWWWWWW#.......",
    "#WWWWWWWW#......",
    "#WWWW#WWWW#.....",
    "#WWW#..#WWWW#...",
    "#WW#....#WWWW#..",
    "#W#......#WWWW#.",
    "##........#WWWW#",
    "#.........#WWWW#",
];

pub struct PointerCursor {
    position: Option<(usize, usize)>,
    background: [BltPixel; CURSOR_WIDTH * CURSOR_HEIGHT],
}

impl PointerCursor {
    pub const fn new() -> Self {
        Self {
            position: None,
            background: [BltPixel::new(0, 0, 0); CURSOR_WIDTH * CURSOR_HEIGHT],
        }
    }

    pub fn hide(&mut self, canvas: &mut GopCanvas) -> uefi::Result {
        if let Some((x, y)) = self.position {
            canvas.gop.blt(BltOp::BufferToVideo {
                buffer: &self.background,
                src: BltRegion::Full,
                dest: (x, y),
                dims: (CURSOR_WIDTH, CURSOR_HEIGHT),
            })?;
            self.position = None;
        }

        Ok(())
    }

    pub fn show(&mut self, canvas: &mut GopCanvas, point: Point) -> uefi::Result {
        if canvas.width < CURSOR_WIDTH || canvas.height < CURSOR_HEIGHT {
            return Ok(());
        }

        let x = (point.x.max(0) as usize).min(canvas.width - CURSOR_WIDTH);
        let y = (point.y.max(0) as usize).min(canvas.height - CURSOR_HEIGHT);
        if self.position == Some((x, y)) {
            return Ok(());
        }

        self.hide(canvas)?;
        canvas.gop.blt(BltOp::VideoToBltBuffer {
            buffer: &mut self.background,
            src: (x, y),
            dest: BltRegion::Full,
            dims: (CURSOR_WIDTH, CURSOR_HEIGHT),
        })?;

        for (row_index, row) in CURSOR_MASK.iter().enumerate() {
            let row = row.as_bytes();
            let mut column = 0;
            while column < CURSOR_WIDTH {
                let pixel = row[column];
                if pixel == b'.' {
                    column += 1;
                    continue;
                }

                let start = column;
                while column < CURSOR_WIDTH && row[column] == pixel {
                    column += 1;
                }

                let color = if pixel == b'#' {
                    Color::new(20, 28, 36)
                } else {
                    Color::new(255, 255, 255)
                };
                canvas.fill_rect(
                    Rect::new(
                        x as i32 + start as i32,
                        y as i32 + row_index as i32,
                        (column - start) as u32,
                        1,
                    ),
                    color,
                )?;
            }
        }

        self.position = Some((x, y));
        Ok(())
    }
}
