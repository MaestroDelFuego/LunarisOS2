#![no_std]
#![no_main]

use myos_filesystem::FileSystem;
use myos_gui_core::{
    Button, ButtonState, Calculator, Color, Point, Rect, StartMenuState, WindowId, WindowManager,
    WindowState,
};
use uefi::boot;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;

mod explorer;
mod framebuffer;
mod gui;
mod input;
mod memory;
mod notepad;
mod serial;
mod storage;
mod wallpaper;

fn redraw_scene<const N: usize>(
    canvas: &mut gui::GopCanvas,
    manager: &WindowManager<N>,
    start_menu: &StartMenuState,
    clock: &[u8; 5],
    launcher: &Button,
    button: &Button,
    button_window: Option<WindowId>,
    notepad_window: Option<WindowId>,
    explorer_window: Option<WindowId>,
    calculator_window: Option<WindowId>,
    notepad: &notepad::Notepad,
    explorer: &explorer::Explorer,
    calculator: &Calculator,
    cursor: &mut gui::PointerCursor,
    pointer_position: Point,
    pointer_available: bool,
    clip: Option<Rect>,
) {
    cursor.hide(canvas).unwrap();
    canvas.set_clip(clip);
    canvas.draw_wallpaper().unwrap();
    canvas.draw_desktop_icon(launcher).unwrap();

    for window in manager.windows_back_to_front() {
        if window.state() == WindowState::Minimized {
            continue;
        }

        let focused = manager.active_id() == Some(window.id());
        canvas.draw_window(window, focused).unwrap();
        if Some(window.id()) == button_window {
            canvas.draw_button(button).unwrap();
        }
        if Some(window.id()) == notepad_window {
            canvas
                .draw_notepad(
                    window,
                    notepad.text(),
                    notepad.cursor(),
                    notepad.status(),
                    notepad.filename(),
                    notepad.dialog_view(),
                    focused,
                )
                .unwrap();
        } else if Some(window.id()) == explorer_window {
            canvas
                .draw_explorer(
                    window,
                    explorer.entries(),
                    explorer.selected(),
                    explorer.visible_start(),
                    explorer.location(),
                    explorer.status(),
                    explorer.dialog(),
                    explorer.edit_name(),
                    explorer.context_menu(),
                )
                .unwrap();
        } else if Some(window.id()) == calculator_window {
            canvas.draw_calculator(window, calculator).unwrap();
        } else if focused && Some(window.id()) == button_window {
            let bounds = window.bounds();
            let text_color = Color::new(35, 47, 61);
            canvas
                .draw_text(
                    bounds.x + 18,
                    bounds.y + 58,
                    "Esc closes; F1 opens another; F2 Start.",
                    text_color,
                )
                .unwrap();
            canvas
                .draw_text(
                    bounds.x + 18,
                    bounds.y + 78,
                    "Tab changes focus; Enter maximizes.",
                    text_color,
                )
                .unwrap();
            canvas
                .draw_text(
                    bounds.x + 18,
                    bounds.y + 98,
                    "Esc closes it; F1 opens a test window.",
                    text_color,
                )
                .unwrap();
        }
    }

    if manager.active_id().is_none() {
        canvas
            .draw_text(
                32,
                140,
                "Enter or Space opens Window Demo.",
                Color::new(255, 255, 255),
            )
            .unwrap();
    }

    let clock = core::str::from_utf8(clock).unwrap_or("--:--");
    canvas.draw_taskbar(manager, start_menu, clock).unwrap();
    canvas.draw_start_menu(start_menu).unwrap();

    canvas.set_clip(None);
    if pointer_available {
        cursor.show(canvas, pointer_position).unwrap();
    }
}

fn launch_explorer(
    manager: &mut WindowManager<5>,
    explorer_window: &mut Option<WindowId>,
    explorer: &mut explorer::Explorer,
    filesystem: &Option<FileSystem<storage::UefiBlockDevice>>,
    width: usize,
    height: usize,
) -> bool {
    explorer.refresh(filesystem.as_ref());
    if let Some(id) = *explorer_window {
        if let Some(window) = manager.window(id) {
            return if window.state() == WindowState::Minimized {
                manager.restore(id).unwrap_or(false)
            } else {
                manager.focus(id).unwrap_or(false)
            };
        }
    }

    let bounds = Rect::new(
        (width / 2).saturating_sub(300) as i32,
        (height / 2).saturating_sub(220) as i32,
        600,
        440,
    );
    let Ok(id) = manager.open("File Explorer", bounds) else {
        return false;
    };
    *explorer_window = Some(id);
    uefi::println!("File Explorer opened ({} items).", explorer.entries().len());
    true
}

fn union_rects(first: Rect, second: Rect) -> Rect {
    let left = i64::from(first.x.min(second.x));
    let top = i64::from(first.y.min(second.y));
    let right = (i64::from(first.x) + i64::from(first.width))
        .max(i64::from(second.x) + i64::from(second.width));
    let bottom = (i64::from(first.y) + i64::from(first.height))
        .max(i64::from(second.y) + i64::from(second.height));
    Rect::new(
        left as i32,
        top as i32,
        (right - left).clamp(0, i64::from(u32::MAX)) as u32,
        (bottom - top).clamp(0, i64::from(u32::MAX)) as u32,
    )
}

fn demo_window_bounds(width: usize, height: usize) -> Rect {
    Rect::new(
        (width / 2).saturating_sub(180) as i32,
        (height / 2).saturating_sub(100) as i32,
        360,
        240,
    )
}

fn launch_window_demo(
    manager: &mut WindowManager<5>,
    button: &mut Button,
    button_window: &mut Option<WindowId>,
    width: usize,
    height: usize,
) -> bool {
    if manager.active_id().is_none() {
        let workspace_bounds = Rect::new(
            (width / 2).saturating_sub(230) as i32,
            (height / 2).saturating_sub(135) as i32,
            440,
            270,
        );
        if manager.open("MyOS workspace", workspace_bounds).is_err() {
            return false;
        }
    }

    let bounds = demo_window_bounds(width, height);
    let Ok(id) = manager.open("Window manager", bounds) else {
        return false;
    };
    *button_window = Some(id);
    button.set_label("GUI test");
    button.set_bounds(Rect::new(bounds.x + 18, bounds.y + 140, 128, 36));
    true
}

fn launch_notepad(
    manager: &mut WindowManager<5>,
    notepad_window: &mut Option<WindowId>,
    notepad: &mut notepad::Notepad,
    filesystem: &mut Option<FileSystem<storage::UefiBlockDevice>>,
    width: usize,
    height: usize,
) -> bool {
    if let Some(id) = *notepad_window {
        if let Some(window) = manager.window(id) {
            return if window.state() == WindowState::Minimized {
                manager.restore(id).unwrap_or(false)
            } else {
                manager.focus(id).unwrap_or(false)
            };
        }
    }

    let bounds = Rect::new(
        (width / 2).saturating_sub(350) as i32,
        (height / 2).saturating_sub(245) as i32,
        700,
        490,
    );
    let Ok(id) = manager.open("Notepad - NOTE.TXT", bounds) else {
        return false;
    };
    *notepad_window = Some(id);

    if !notepad.is_initialized() {
        if let Err(error) = notepad.load(filesystem.as_mut()) {
            report_notepad_error("open", error);
        }
    }
    true
}

fn launch_selected_start_item(
    manager: &mut WindowManager<5>,
    start_menu: &mut StartMenuState,
    button: &mut Button,
    button_window: &mut Option<WindowId>,
    notepad_window: &mut Option<WindowId>,
    explorer_window: &mut Option<WindowId>,
    calculator_window: &mut Option<WindowId>,
    notepad: &mut notepad::Notepad,
    explorer: &mut explorer::Explorer,
    filesystem: &mut Option<FileSystem<storage::UefiBlockDevice>>,
    width: usize,
    height: usize,
) -> bool {
    let Some(selection) = start_menu.activate() else {
        return false;
    };
    start_menu.close();
    match selection {
        0 => launch_window_demo(manager, button, button_window, width, height),
        1 => launch_notepad(manager, notepad_window, notepad, filesystem, width, height),
        2 => launch_explorer(
            manager,
            explorer_window,
            explorer,
            filesystem,
            width,
            height,
        ),
        3 => launch_calculator(manager, calculator_window, width, height),
        _ => false,
    }
}

fn launch_calculator(
    manager: &mut WindowManager<5>,
    calculator_window: &mut Option<WindowId>,
    width: usize,
    height: usize,
) -> bool {
    if let Some(id) = *calculator_window {
        if let Some(window) = manager.window(id) {
            return if window.state() == WindowState::Minimized {
                manager.restore(id).unwrap_or(false)
            } else {
                manager.focus(id).unwrap_or(false)
            };
        }
    }
    let bounds = Rect::new(
        (width / 2).saturating_sub(180) as i32,
        (height / 2).saturating_sub(210) as i32,
        360,
        420,
    );
    let Ok(id) = manager.open("Calculator", bounds) else {
        uefi::println!("Calculator could not open: no window slot available.");
        return false;
    };
    *calculator_window = Some(id);
    uefi::println!("Calculator opened.");
    true
}

fn open_selected_explorer_file(
    manager: &mut WindowManager<5>,
    notepad_window: &mut Option<WindowId>,
    notepad: &mut notepad::Notepad,
    explorer: &mut explorer::Explorer,
    filesystem: &mut Option<FileSystem<storage::UefiBlockDevice>>,
    width: usize,
    height: usize,
) -> bool {
    if filesystem.is_none() {
        explorer.set_status("No MyOS filesystem mounted");
        uefi::println!("Explorer open failed: no MyOS filesystem mounted.");
        return true;
    }
    let Some(entry) = explorer.selected_entry() else {
        explorer.set_status("Select an item to open");
        return false;
    };
    if entry.kind == myos_filesystem::EntryKind::Directory {
        return match explorer.enter_selected_directory(filesystem.as_ref()) {
            Ok(changed) => {
                if changed {
                    uefi::println!("Explorer navigated to {}.", explorer.location());
                }
                changed
            }
            Err(error) => {
                uefi::println!("Explorer could not enter folder: {:?}.", error);
                explorer.set_status("Could not open folder; see serial log");
                true
            }
        };
    }
    let mut name = [0; myos_filesystem::MAX_NAME_BYTES];
    let name_len = entry.name_len as usize;
    name[..name_len].copy_from_slice(&entry.name[..name_len]);
    let name = core::str::from_utf8(&name[..name_len]).unwrap_or("");
    if !launch_notepad(manager, notepad_window, notepad, filesystem, width, height) {
        explorer.set_status("Could not open Notepad");
        return false;
    }
    if let Err(error) = notepad.begin_open_at(
        filesystem.as_ref(),
        explorer.current_directory(),
        Some(name),
    ) {
        report_notepad_error("Explorer open", error);
        explorer.set_status("Could not list files");
        return true;
    }
    let selected = notepad
        .dialog_view()
        .and_then(|view| view.files.iter().position(|entry| entry.name() == name));
    if let Some(index) = selected {
        notepad.dialog_select(index);
        match notepad.dialog_accept(filesystem.as_mut()) {
            Ok(true) => {
                uefi::println!("Explorer opened {} in Notepad.", name);
                true
            }
            Ok(false) => true,
            Err(error) => {
                report_notepad_error("Explorer open", error);
                explorer.set_status("Could not open selected file");
                true
            }
        }
    } else {
        notepad.dismiss_dialog();
        explorer.set_status("File is no longer on this volume; press F5");
        true
    }
}

fn handle_explorer_key(
    key: uefi::proto::console::text::Key,
    active_window: Option<WindowId>,
    explorer_window: Option<WindowId>,
    manager: &mut WindowManager<5>,
    notepad_window: &mut Option<WindowId>,
    notepad: &mut notepad::Notepad,
    explorer: &mut explorer::Explorer,
    filesystem: &mut Option<FileSystem<storage::UefiBlockDevice>>,
    width: usize,
    height: usize,
) -> Option<bool> {
    use uefi::proto::console::text::{Key, ScanCode};

    if explorer_window.is_none() || active_window != explorer_window {
        return None;
    }
    if explorer.context_menu().is_some() {
        return match key {
            Key::Special(ScanCode::ESCAPE) => Some(explorer.close_context_menu()),
            _ => Some(false),
        };
    }
    if explorer.dialog().is_some() {
        match key {
            Key::Special(ScanCode::ESCAPE) => return Some(explorer.cancel_dialog()),
            Key::Printable(character) if character == '\r' || character == '\n' => {
                let mode = explorer.dialog();
                match explorer.accept_dialog(filesystem.as_mut()) {
                    Ok(true) => uefi::println!(
                        "Explorer {:?} operation completed in {}.",
                        mode,
                        explorer.location()
                    ),
                    Ok(false) => {}
                    Err(error) => uefi::println!("Explorer operation failed: {:?}.", error),
                }
                return Some(true);
            }
            Key::Printable(character) if char::from(character) == '\u{8}' => {
                return Some(explorer.edit_backspace())
            }
            Key::Printable(character) => {
                explorer.edit_insert(char::from(character));
                return Some(true);
            }
            _ => return Some(false),
        }
    }
    match key {
        Key::Special(ScanCode::UP) => Some(explorer.move_selection(-1)),
        Key::Special(ScanCode::DOWN) => Some(explorer.move_selection(1)),
        Key::Special(ScanCode::FUNCTION_5) => {
            explorer.refresh(filesystem.as_ref());
            Some(true)
        }
        Key::Special(ScanCode::FUNCTION_2) => {
            explorer.begin_create();
            Some(true)
        }
        Key::Special(ScanCode::FUNCTION_4) => {
            explorer.begin_create_directory();
            Some(true)
        }
        Key::Special(ScanCode::FUNCTION_3) => {
            explorer.begin_rename();
            Some(true)
        }
        Key::Special(ScanCode::DELETE) => {
            explorer.begin_delete();
            Some(true)
        }
        Key::Special(ScanCode::FUNCTION_6) => {
            explorer.begin_copy();
            Some(true)
        }
        Key::Special(ScanCode::FUNCTION_7) => {
            explorer.begin_paste();
            Some(true)
        }
        Key::Printable(character) if character == '\r' || character == '\n' => {
            uefi::println!("Explorer open requested.");
            Some(open_selected_explorer_file(
                manager,
                notepad_window,
                notepad,
                explorer,
                filesystem,
                width,
                height,
            ))
        }
        Key::Printable(character) if char::from(character) == '\u{8}' => {
            match explorer.go_parent(filesystem.as_ref()) {
                Ok(changed) => {
                    if changed {
                        uefi::println!("Explorer navigated to {}.", explorer.location());
                    }
                    Some(changed)
                }
                Err(error) => {
                    uefi::println!("Explorer could not return to parent folder: {:?}.", error);
                    explorer.set_status("Could not return to parent; see serial log");
                    Some(true)
                }
            }
        }
        _ => None,
    }
}

fn handle_calculator_key(
    key: uefi::proto::console::text::Key,
    active_window: Option<WindowId>,
    calculator_window: Option<WindowId>,
    calculator: &mut Calculator,
) -> Option<bool> {
    use uefi::proto::console::text::{Key, ScanCode};

    if calculator_window.is_none() || active_window != calculator_window {
        return None;
    }
    match key {
        Key::Special(ScanCode::ESCAPE) => None,
        Key::Special(ScanCode::DELETE) => Some(calculator.press('\u{8}')),
        Key::Printable(character) => {
            let character = char::from(character);
            if character.is_ascii_digit()
                || matches!(
                    character,
                    '.' | '+' | '-' | '*' | '/' | '=' | '\r' | '\n' | '\u{8}' | 'c' | 'C'
                )
            {
                let changed = calculator.press(character);
                if matches!(character, '=' | '\r' | '\n') {
                    uefi::println!("Calculator result: {}.", calculator.display());
                }
                Some(changed)
            } else if character == ' ' {
                Some(false)
            } else {
                None
            }
        }
        Key::Special(_) => None,
    }
}

fn handle_notepad_key(
    key: uefi::proto::console::text::Key,
    active_window: Option<WindowId>,
    notepad_window: Option<WindowId>,
    notepad: &mut notepad::Notepad,
    filesystem: &mut Option<FileSystem<storage::UefiBlockDevice>>,
) -> Option<bool> {
    use uefi::proto::console::text::{Key, ScanCode};

    if active_window != notepad_window {
        return None;
    }

    if notepad.has_dialog() {
        match key {
            Key::Special(ScanCode::ESCAPE) => {
                return Some(notepad.dialog_cancel());
            }
            Key::Special(ScanCode::UP) => return Some(notepad.dialog_move_selection(-1)),
            Key::Special(ScanCode::DOWN) => return Some(notepad.dialog_move_selection(1)),
            Key::Printable(character) => match char::from(character) {
                '\r' | '\n' => {
                    match notepad.dialog_accept(filesystem.as_mut()) {
                        Ok(_) => {}
                        Err(error) => report_notepad_error("dialog action", error),
                    }
                    return Some(true);
                }
                '\u{8}' => return Some(notepad.dialog_backspace()),
                character => return Some(notepad.dialog_insert(character)),
            },
            _ => return Some(false),
        }
    }

    match key {
        Key::Special(ScanCode::FUNCTION_5) => {
            if let Err(error) = notepad.begin_open(filesystem.as_ref()) {
                report_notepad_error("open dialog", error);
            }
            Some(true)
        }
        Key::Special(ScanCode::FUNCTION_6) => {
            if let Err(error) = notepad.save(filesystem.as_mut()) {
                report_notepad_error("save", error);
            }
            Some(true)
        }
        Key::Special(ScanCode::FUNCTION_7) => {
            if let Err(error) = notepad.begin_save_as(filesystem.as_ref()) {
                report_notepad_error("save-as dialog", error);
            }
            Some(true)
        }
        Key::Special(ScanCode::ESCAPE) => {
            notepad.request_close();
            Some(true)
        }
        Key::Printable(character) => match char::from(character) {
            '\u{8}' => Some(notepad.backspace()),
            '\r' | '\n' => Some(notepad.insert('\n')),
            character => Some(notepad.insert(character)),
        },
        Key::Special(ScanCode::LEFT) => Some(notepad.move_left()),
        Key::Special(ScanCode::RIGHT) => Some(notepad.move_right()),
        Key::Special(ScanCode::UP) => Some(notepad.move_up()),
        Key::Special(ScanCode::DOWN) => Some(notepad.move_down()),
        Key::Special(ScanCode::HOME) => Some(notepad.move_home()),
        Key::Special(ScanCode::END) => Some(notepad.move_end()),
        Key::Special(ScanCode::DELETE) => Some(notepad.delete()),
        _ => None,
    }
}

fn report_notepad_error(operation: &str, error: notepad::NotepadError<uefi::Error>) {
    match error {
        notepad::NotepadError::StorageUnavailable => {
            uefi::println!("Notepad {} failed: no MyOS filesystem mounted.", operation)
        }
        notepad::NotepadError::Filesystem(error) => {
            uefi::println!(
                "Notepad {} failed: filesystem error: {:?}",
                operation,
                error
            )
        }
        notepad::NotepadError::UnsupportedText(error) => {
            uefi::println!(
                "Notepad {} failed: unsupported text: {:?}",
                operation,
                error
            )
        }
    }
}

fn current_clock() -> [u8; 5] {
    let Ok(time) = uefi::runtime::get_time() else {
        return *b"--:--";
    };

    let hour = time.hour();
    let minute = time.minute();
    [
        b'0' + hour / 10,
        b'0' + hour % 10,
        b':',
        b'0' + minute / 10,
        b'0' + minute % 10,
    ]
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    uefi::println!("MYOS booting...");
    uefi::println!("Milestones 9-10: Notepad and hierarchical Explorer");
    uefi::println!("Boot path: UEFI/QEMU foundation");

    let fb = framebuffer::default_target();
    uefi::println!(
        "Framebuffer target: {}x{} @ {} bpp",
        fb.width,
        fb.height,
        fb.bpp
    );

    let handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();

    let mode_tuple = framebuffer::choose_mode(
        fb.width,
        fb.height,
        gop.modes().map(|mode| {
            let (width, height) = mode.info().resolution();
            (width as u32, height as u32)
        }),
    );

    let selected_mode = gop
        .modes()
        .find(|mode| {
            let (width, height) = mode.info().resolution();
            (width as u32, height as u32) == mode_tuple
        })
        .unwrap();

    gop.set_mode(&selected_mode).unwrap();

    let (width, height) = gop.current_mode_info().resolution();
    wallpaper::draw(&mut gop, width, height, None).unwrap();

    let memory = memory::detect_memory();
    uefi::println!(
        "Memory layout: {} KiB total, {} KiB usable",
        memory.total_kib,
        memory.usable_kib
    );

    serial::log("Graphics output protocol configured successfully.");
    serial::log("canvas.png wallpaper rendered.");

    let mut filesystem = match storage::mount_existing() {
        Ok(Some(filesystem)) => {
            uefi::println!("MyOS filesystem mounted from UEFI block storage.");
            Some(filesystem)
        }
        Ok(None) => {
            uefi::println!("No MyOS filesystem found; disks were left unchanged.");
            None
        }
        Err(error) => {
            match error {
                storage::StorageError::Locate(error) => {
                    uefi::println!("Storage discovery failed: {:?}", error)
                }
                storage::StorageError::Filesystem(error) => {
                    uefi::println!("Storage mount failed: {:?}", error)
                }
            }
            None
        }
    };

    uefi::println!("Graphics initialized. Framebuffer is active.");
    uefi::println!("Keyboard input initialized.");
    uefi::println!("Kernel initialization complete for milestone 9.");

    let mut pointer = input::MouseDevice::open();
    if let Some(device) = pointer.as_ref() {
        let (relative, absolute, ps2) = device.device_counts();
        uefi::println!(
            "Mouse input initialized ({} relative, {} absolute pointer devices; PS/2 fallback: {}).",
            relative,
            absolute,
            if ps2 { "ready" } else { "unavailable" }
        );
    } else {
        uefi::println!("No UEFI mouse protocol found; mouse input is unavailable.");
    }
    let work_area = Rect::new(0, 0, width as u32, height as u32);
    let mut manager = WindowManager::<5>::new(work_area);
    let mut start_menu = StartMenuState::new(gui::START_MENU_ITEM_COUNT);
    let mut clock = current_clock();
    let mut button_window = None;
    let mut notepad_window = None;
    let mut explorer_window = None;
    let mut calculator_window = None;
    let mut notepad = notepad::Notepad::new();
    let mut explorer = explorer::Explorer::new();
    let mut calculator = Calculator::new();
    let mut button = Button::new(Rect::new(0, 0, 128, 36), "GUI test");
    let icon_bounds = Rect::new(32, 32, 112, 100);
    let mut launcher = Button::new(icon_bounds, "Window Demo");
    launcher.update_pointer(
        Point {
            x: icon_bounds.x + icon_bounds.width as i32 / 2,
            y: icon_bounds.y + icon_bounds.height as i32 / 2,
        },
        false,
    );
    let mut pointer_position = Point {
        x: (width / 2) as i32,
        y: (height / 2) as i32,
    };
    let mut pointer_visible = false;
    let mut canvas = gui::GopCanvas::new(&mut gop, width, height);
    let mut cursor = gui::PointerCursor::new();
    redraw_scene(
        &mut canvas,
        &manager,
        &start_menu,
        &clock,
        &launcher,
        &button,
        button_window,
        notepad_window,
        explorer_window,
        calculator_window,
        &notepad,
        &explorer,
        &calculator,
        &mut cursor,
        pointer_position,
        pointer_visible,
        None,
    );
    let mut left_was_down = false;
    let mut right_was_down = false;
    let mut left_press_consumed = false;
    let mut clock_ticks = 0;
    let mut event_ticks = 0u64;

    loop {
        event_ticks = event_ticks.wrapping_add(1);
        if let Some(key) = input::read_key() {
            let mut changed = false;
            let explorer_action = if start_menu.is_open() {
                None
            } else {
                handle_explorer_key(
                    key,
                    manager.active_id(),
                    explorer_window,
                    &mut manager,
                    &mut notepad_window,
                    &mut notepad,
                    &mut explorer,
                    &mut filesystem,
                    width,
                    height,
                )
            };
            let editor_action = if explorer_action.is_some() || start_menu.is_open() {
                None
            } else {
                handle_notepad_key(
                    key,
                    manager.active_id(),
                    notepad_window,
                    &mut notepad,
                    &mut filesystem,
                )
            };
            let calculator_action = if explorer_action.is_some()
                || editor_action.is_some()
                || start_menu.is_open()
            {
                None
            } else {
                handle_calculator_key(key, manager.active_id(), calculator_window, &mut calculator)
            };
            if let Some(explorer_changed) = explorer_action {
                changed = explorer_changed;
            } else if let Some(editor_changed) = editor_action {
                changed = editor_changed;
                if notepad.take_close_request() {
                    if let Some(id) = notepad_window {
                        changed |= manager.close(id).is_ok();
                        notepad_window = None;
                    }
                }
            } else if let Some(calculator_changed) = calculator_action {
                changed = calculator_changed;
            } else if let Some(action) = input::window_action(key) {
                match action {
                    input::WindowAction::MoveUp if start_menu.is_open() => {
                        changed = start_menu.move_selection(-1);
                    }
                    input::WindowAction::MoveDown if start_menu.is_open() => {
                        changed = start_menu.move_selection(1);
                    }
                    input::WindowAction::MoveLeft
                    | input::WindowAction::MoveRight
                    | input::WindowAction::MoveUp
                    | input::WindowAction::MoveDown => {
                        if let Some(window) = manager.active_window().copied() {
                            let (dx, dy) = match action {
                                input::WindowAction::MoveLeft => (-16, 0),
                                input::WindowAction::MoveRight => (16, 0),
                                input::WindowAction::MoveUp => (0, -16),
                                input::WindowAction::MoveDown => (0, 16),
                                _ => (0, 0),
                            };
                            let max_x = width.saturating_sub(window.bounds().width as usize) as i32;
                            let max_y =
                                height.saturating_sub(window.bounds().height as usize) as i32;
                            let x = window.bounds().x.saturating_add(dx).clamp(0, max_x);
                            let y = window.bounds().y.saturating_add(dy).clamp(0, max_y);
                            changed = manager.move_to(window.id(), x, y).is_ok();
                        }
                    }
                    input::WindowAction::NextWindow => {
                        changed = manager.focus_next().is_some();
                    }
                    input::WindowAction::ToggleMaximize => {
                        if start_menu.is_open() {
                            changed = launch_selected_start_item(
                                &mut manager,
                                &mut start_menu,
                                &mut button,
                                &mut button_window,
                                &mut notepad_window,
                                &mut explorer_window,
                                &mut calculator_window,
                                &mut notepad,
                                &mut explorer,
                                &mut filesystem,
                                width,
                                height,
                            );
                        } else if let Some(window) = manager.active_window().copied() {
                            changed = if window.state() == WindowState::Maximized {
                                manager.restore(window.id()).unwrap_or(false)
                            } else {
                                manager.maximize(window.id()).unwrap_or(false)
                            };
                        } else {
                            changed = launch_window_demo(
                                &mut manager,
                                &mut button,
                                &mut button_window,
                                width,
                                height,
                            );
                        }
                    }
                    input::WindowAction::Close => {
                        if start_menu.close() {
                            changed = true;
                        } else if let Some(id) = manager.active_id() {
                            changed = manager.close(id).is_ok();
                            if changed && notepad_window == Some(id) {
                                notepad_window = None;
                            }
                            if changed && explorer_window == Some(id) {
                                explorer_window = None;
                            }
                            if changed && calculator_window == Some(id) {
                                calculator_window = None;
                            }
                        }
                    }
                    input::WindowAction::OpenTestWindow => {
                        start_menu.close();
                        if manager.active_id().is_none() {
                            changed = launch_window_demo(
                                &mut manager,
                                &mut button,
                                &mut button_window,
                                width,
                                height,
                            );
                        } else {
                            changed = manager
                                .open("Window manager", demo_window_bounds(width, height))
                                .is_ok();
                        }
                    }
                    input::WindowAction::ActivateControl => {
                        if start_menu.is_open() {
                            changed = launch_selected_start_item(
                                &mut manager,
                                &mut start_menu,
                                &mut button,
                                &mut button_window,
                                &mut notepad_window,
                                &mut explorer_window,
                                &mut calculator_window,
                                &mut notepad,
                                &mut explorer,
                                &mut filesystem,
                                width,
                                height,
                            );
                        } else if let Some(button_window_id) = button_window {
                            if manager.active_id() == Some(button_window_id) && button.activate() {
                                button.set_label(if button.label() == "GUI test" {
                                    "Clicked!"
                                } else {
                                    "GUI test"
                                });
                                changed = true;
                            }
                        } else if launcher.activate() {
                            changed = launch_window_demo(
                                &mut manager,
                                &mut button,
                                &mut button_window,
                                width,
                                height,
                            );
                        }
                    }
                    input::WindowAction::ToggleStartMenu => {
                        changed = start_menu.toggle();
                    }
                }
            } else {
                canvas
                    .fill_rect(Rect::new(8, 8, 16, 16), Color::new(0xff, 0xd0, 0x20))
                    .unwrap();
            }

            if changed {
                if let Some(window) = button_window.and_then(|id| manager.window(id)) {
                    let bounds = window.bounds();
                    button.set_bounds(Rect::new(bounds.x + 18, bounds.y + 140, 128, 36));
                }
                redraw_scene(
                    &mut canvas,
                    &manager,
                    &start_menu,
                    &clock,
                    &launcher,
                    &button,
                    button_window,
                    notepad_window,
                    explorer_window,
                    calculator_window,
                    &notepad,
                    &explorer,
                    &calculator,
                    &mut cursor,
                    pointer_position,
                    pointer_visible,
                    None,
                );
            }
        }

        if let Some(state) = pointer
            .as_mut()
            .and_then(|device| device.read(width, height))
        {
            let pointer_moved = state.movement_x != 0
                || state.movement_y != 0
                || state.left_down
                || state.right_down
                || state.position.is_some_and(|position| {
                    position.x != pointer_position.x || position.y != pointer_position.y
                });
            if pointer_moved && !pointer_visible {
                uefi::println!("Mouse input received.");
                pointer_visible = true;
            }
            cursor.hide(&mut canvas).unwrap();
            if let Some(position) = state.position {
                pointer_position.x = position.x;
                pointer_position.y = position.y;
            } else {
                pointer_position.x = pointer_position
                    .x
                    .saturating_add(state.movement_x)
                    .clamp(0, width.saturating_sub(1) as i32);
                pointer_position.y = pointer_position
                    .y
                    .saturating_add(state.movement_y)
                    .clamp(0, height.saturating_sub(1) as i32);
            }

            let left_is_down = state.left_down;
            let right_is_down = state.right_down;
            let right_was_down_previous = right_was_down;
            right_was_down = right_is_down;
            let mut changed = false;
            let mut taskbar_handled =
                !left_is_down && left_was_down && core::mem::take(&mut left_press_consumed);
            let mut drag_damage = None;
            if right_is_down && !right_was_down_previous {
                if let Some(id) = manager.window_at(pointer_position) {
                    if explorer_window == Some(id) {
                        if let Some(window) = manager.window(id) {
                            let row = (0..myos_filesystem::MAX_FILES).find(|row| {
                                gui::explorer_row_bounds(window.bounds(), *row)
                                    .contains(pointer_position)
                            });
                            let index = row
                                .map(|row| explorer.visible_start() + row)
                                .filter(|index| *index < explorer.entries().len());
                            if let Some(index) = index {
                                changed |= manager.focus(id).unwrap_or(false);
                                changed |= explorer.open_context_menu(
                                    index,
                                    pointer_position.x,
                                    pointer_position.y,
                                );
                                if let Some(file) = explorer.selected_entry() {
                                    uefi::println!(
                                        "Explorer context menu opened for {}.",
                                        file.name()
                                    );
                                }
                            } else {
                                changed |= explorer.close_context_menu();
                            }
                        }
                        taskbar_handled = true;
                    } else {
                        changed |= explorer.close_context_menu();
                    }
                } else {
                    changed |= explorer.close_context_menu();
                }
            }
            if start_menu.is_open() {
                if let Some(index) = (0..gui::START_MENU_ITEM_COUNT).find(|index| {
                    gui::start_menu_entry_bounds(*index, height).contains(pointer_position)
                }) {
                    changed |= start_menu.select_item(index);
                }
            }
            if !left_is_down && left_was_down {
                if start_menu.is_open()
                    && (0..gui::START_MENU_ITEM_COUNT).any(|index| {
                        gui::start_menu_entry_bounds(index, height).contains(pointer_position)
                    })
                {
                    if let Some(index) = (0..gui::START_MENU_ITEM_COUNT).find(|index| {
                        gui::start_menu_entry_bounds(*index, height).contains(pointer_position)
                    }) {
                        start_menu.select_item(index);
                        changed = launch_selected_start_item(
                            &mut manager,
                            &mut start_menu,
                            &mut button,
                            &mut button_window,
                            &mut notepad_window,
                            &mut explorer_window,
                            &mut calculator_window,
                            &mut notepad,
                            &mut explorer,
                            &mut filesystem,
                            width,
                            height,
                        );
                    }
                    taskbar_handled = true;
                } else if gui::taskbar_start_bounds(height).contains(pointer_position) {
                    changed = start_menu.toggle();
                    taskbar_handled = true;
                } else if pointer_position.y >= height.saturating_sub(40) as i32 {
                    let task_window =
                        manager
                            .windows_back_to_front()
                            .enumerate()
                            .find_map(|(index, window)| {
                                gui::taskbar_window_bounds(index, height)
                                    .contains(pointer_position)
                                    .then_some((window.id(), window.state()))
                            });
                    if let Some((id, state)) = task_window {
                        changed = if state == WindowState::Minimized {
                            manager.restore(id).unwrap_or(false)
                        } else {
                            manager.focus(id).is_ok()
                        };
                        taskbar_handled = true;
                    }
                } else if start_menu.is_open() {
                    changed |= start_menu.close();
                    taskbar_handled = true;
                }
            }

            if !taskbar_handled && left_is_down && !left_was_down {
                if let Some(id) = manager.window_at(pointer_position) {
                    if notepad_window == Some(id) {
                        if let Some(window) = manager.window(id).copied() {
                            if gui::notepad_open_bounds(window.bounds()).contains(pointer_position)
                            {
                                if let Err(error) = notepad.begin_open(filesystem.as_ref()) {
                                    report_notepad_error("open dialog", error);
                                }
                                changed = true;
                                taskbar_handled = true;
                            } else if gui::notepad_save_bounds(window.bounds())
                                .contains(pointer_position)
                            {
                                match notepad.save(filesystem.as_mut()) {
                                    Ok(()) => {
                                        uefi::println!("Notepad saved {}.", notepad.filename());
                                    }
                                    Err(error) => report_notepad_error("save", error),
                                }
                                changed = true;
                                taskbar_handled = true;
                            } else if gui::notepad_save_as_bounds(window.bounds())
                                .contains(pointer_position)
                            {
                                if let Err(error) = notepad.begin_save_as(filesystem.as_ref()) {
                                    report_notepad_error("save-as dialog", error);
                                }
                                changed = true;
                                taskbar_handled = true;
                            }
                            if taskbar_handled {
                                left_press_consumed = true;
                            }
                        }
                    }
                }
            }

            if !taskbar_handled
                && !left_is_down
                && left_was_down
                && explorer.context_menu().is_some()
                && explorer
                    .context_menu()
                    .and_then(|position| {
                        gui::explorer_context_menu_item_at(
                            position,
                            pointer_position,
                            width,
                            height,
                        )
                    })
                    .is_none()
            {
                changed |= explorer.close_context_menu();
                taskbar_handled = true;
            }

            if !taskbar_handled {
                if !left_is_down && left_was_down {
                    if let Some(id) = manager.window_at(pointer_position) {
                        if let Some(window) = manager.window(id).copied() {
                            changed |= manager.focus(id).unwrap_or(false);
                            if let Some(control) =
                                gui::window_control_at(window.bounds(), pointer_position)
                            {
                                match control {
                                    gui::WindowControl::Close => {
                                        if notepad_window == Some(id) {
                                            notepad.dismiss_dialog();
                                            if notepad.request_close() {
                                                notepad.take_close_request();
                                                if manager.close(id).is_ok() {
                                                    notepad_window = None;
                                                    changed = true;
                                                }
                                            } else {
                                                changed = true;
                                            }
                                        } else if manager.close(id).is_ok() {
                                            changed = true;
                                            if explorer_window == Some(id) {
                                                explorer_window = None;
                                            }
                                            if calculator_window == Some(id) {
                                                calculator_window = None;
                                            }
                                        }
                                    }
                                    gui::WindowControl::Maximize => {
                                        changed |= if window.state() == WindowState::Maximized {
                                            manager.restore(id).unwrap_or(false)
                                        } else {
                                            manager.maximize(id).unwrap_or(false)
                                        };
                                    }
                                    gui::WindowControl::Minimize => {
                                        changed |= manager.minimize(id).unwrap_or(false);
                                    }
                                }
                                taskbar_handled = true;
                            } else if notepad_window == Some(id) {
                                if notepad.has_dialog() {
                                    let dialog = notepad.dialog_view().unwrap();
                                    if gui::file_dialog_accept_bounds(window.bounds())
                                        .contains(pointer_position)
                                    {
                                        match notepad.dialog_accept(filesystem.as_mut()) {
                                            Ok(_) => {}
                                            Err(error) => {
                                                report_notepad_error("dialog action", error)
                                            }
                                        }
                                        if notepad.take_close_request() && manager.close(id).is_ok()
                                        {
                                            notepad_window = None;
                                        }
                                    } else if gui::file_dialog_cancel_bounds(window.bounds())
                                        .contains(pointer_position)
                                    {
                                        notepad.dialog_cancel();
                                    } else if matches!(dialog.mode, notepad::FileDialogMode::SaveAs)
                                    {
                                        let row = (0..7).find(|row| {
                                            gui::file_dialog_row_bounds(window.bounds(), *row)
                                                .contains(pointer_position)
                                        });
                                        if let Some(row) = row {
                                            notepad.dialog_select(dialog.visible_start + row);
                                        }
                                    } else if dialog.mode == notepad::FileDialogMode::Open {
                                        let row = (0..7).find(|row| {
                                            gui::file_dialog_row_bounds(window.bounds(), *row)
                                                .contains(pointer_position)
                                        });
                                        if let Some(row) = row {
                                            notepad.dialog_select(dialog.visible_start + row);
                                        }
                                    }
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::notepad_open_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    if let Err(error) = notepad.begin_open(filesystem.as_ref()) {
                                        report_notepad_error("open dialog", error);
                                    }
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::notepad_save_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    if let Err(error) = notepad.save(filesystem.as_mut()) {
                                        report_notepad_error("save", error);
                                    }
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::notepad_save_as_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    if let Err(error) = notepad.begin_save_as(filesystem.as_ref()) {
                                        report_notepad_error("save-as dialog", error);
                                    }
                                    changed = true;
                                    taskbar_handled = true;
                                } else if let Some(cursor) = gui::notepad_caret_at(
                                    window.bounds(),
                                    notepad.text(),
                                    notepad.cursor(),
                                    pointer_position,
                                ) {
                                    changed |= notepad.set_cursor(cursor);
                                }
                            } else if explorer_window == Some(id) {
                                if let Some(menu_position) = explorer.context_menu() {
                                    let item = gui::explorer_context_menu_item_at(
                                        menu_position,
                                        pointer_position,
                                        width,
                                        height,
                                    );
                                    let action =
                                        item.and_then(|item| explorer.context_action(item));
                                    explorer.close_context_menu();
                                    match action {
                                        Some(explorer::ExplorerContextAction::New) => {
                                            explorer.begin_create();
                                            changed = true;
                                        }
                                        Some(explorer::ExplorerContextAction::Open) => {
                                            changed |= open_selected_explorer_file(
                                                &mut manager,
                                                &mut notepad_window,
                                                &mut notepad,
                                                &mut explorer,
                                                &mut filesystem,
                                                width,
                                                height,
                                            );
                                        }
                                        Some(explorer::ExplorerContextAction::Rename) => {
                                            explorer.begin_rename();
                                            changed = true;
                                        }
                                        Some(explorer::ExplorerContextAction::Delete) => {
                                            explorer.begin_delete();
                                            changed = true;
                                        }
                                        Some(explorer::ExplorerContextAction::Copy) => {
                                            explorer.begin_copy();
                                            changed = true;
                                        }
                                        Some(explorer::ExplorerContextAction::Paste) => {
                                            explorer.begin_paste();
                                            changed = true;
                                        }
                                        Some(explorer::ExplorerContextAction::Properties) => {
                                            changed = explorer.begin_properties();
                                            if changed {
                                                if let Some(file) = explorer.selected_entry() {
                                                    uefi::println!(
                                                        "Explorer properties opened for {}.",
                                                        file.name()
                                                    );
                                                }
                                            }
                                        }
                                        None => changed = true,
                                    }
                                    taskbar_handled = true;
                                } else if explorer.dialog().is_some() {
                                    if gui::explorer_dialog_accept_bounds(window.bounds())
                                        .contains(pointer_position)
                                    {
                                        let mode = explorer.dialog();
                                        match explorer.accept_dialog(filesystem.as_mut()) {
                                            Ok(true) => uefi::println!(
                                                "Explorer {:?} operation completed in {}.",
                                                mode,
                                                explorer.location()
                                            ),
                                            Ok(false) => {}
                                            Err(error) => uefi::println!(
                                                "Explorer operation failed: {:?}.",
                                                error
                                            ),
                                        }
                                    } else if gui::explorer_dialog_cancel_bounds(window.bounds())
                                        .contains(pointer_position)
                                    {
                                        explorer.cancel_dialog();
                                    }
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::explorer_parent_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    match explorer.go_parent(filesystem.as_ref()) {
                                        Ok(navigated) => {
                                            changed |= navigated;
                                            if navigated {
                                                uefi::println!(
                                                    "Explorer navigated to {}.",
                                                    explorer.location()
                                                );
                                            }
                                        }
                                        Err(error) => {
                                            uefi::println!(
                                                "Explorer could not return to parent folder: {:?}.",
                                                error
                                            );
                                            explorer.set_status(
                                                "Could not return to parent; see serial log",
                                            );
                                            changed = true;
                                        }
                                    }
                                    taskbar_handled = true;
                                } else if gui::explorer_create_directory_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    explorer.begin_create_directory();
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::explorer_create_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    explorer.begin_create();
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::explorer_rename_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    explorer.begin_rename();
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::explorer_delete_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    explorer.begin_delete();
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::explorer_copy_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    explorer.begin_copy();
                                    changed = true;
                                    taskbar_handled = true;
                                } else if gui::explorer_paste_bounds(window.bounds())
                                    .contains(pointer_position)
                                {
                                    explorer.begin_paste();
                                    changed = true;
                                    taskbar_handled = true;
                                } else if let Some(row) =
                                    (0..myos_filesystem::MAX_FILES).find(|row| {
                                        gui::explorer_row_bounds(window.bounds(), *row)
                                            .contains(pointer_position)
                                    })
                                {
                                    let index = explorer.visible_start() + row;
                                    let open_file = explorer.click_row(index, event_ticks);
                                    changed = true;
                                    if open_file {
                                        changed |= open_selected_explorer_file(
                                            &mut manager,
                                            &mut notepad_window,
                                            &mut notepad,
                                            &mut explorer,
                                            &mut filesystem,
                                            width,
                                            height,
                                        );
                                    }
                                    taskbar_handled = true;
                                }
                            } else if calculator_window == Some(id) {
                                if let Some(key) =
                                    gui::calculator_key_at(window.bounds(), pointer_position)
                                {
                                    calculator.press(key);
                                    if key == '=' {
                                        uefi::println!(
                                            "Calculator result: {}.",
                                            calculator.display()
                                        );
                                    }
                                    changed = true;
                                }
                                taskbar_handled = true;
                            }
                        }
                    }
                }

                if taskbar_handled {
                    left_was_down = left_is_down;
                    if changed {
                        redraw_scene(
                            &mut canvas,
                            &manager,
                            &start_menu,
                            &clock,
                            &launcher,
                            &button,
                            button_window,
                            notepad_window,
                            explorer_window,
                            calculator_window,
                            &notepad,
                            &explorer,
                            &calculator,
                            &mut cursor,
                            pointer_position,
                            pointer_visible,
                            None,
                        );
                    } else if pointer_visible {
                        cursor.show(&mut canvas, pointer_position).unwrap();
                    }
                    continue;
                }

                let pointer_over_window = manager.window_at(pointer_position).is_some();
                if !pointer_over_window {
                    let update = launcher.update_pointer(pointer_position, left_is_down);
                    changed |= update.state_changed;
                    if update.clicked && launcher.activate() {
                        changed |= launch_window_demo(
                            &mut manager,
                            &mut button,
                            &mut button_window,
                            width,
                            height,
                        );
                    }
                } else if let Some(button_window_id) = button_window {
                    if manager.active_id() == Some(button_window_id) {
                        let update = button.update_pointer(pointer_position, left_is_down);
                        changed |= update.state_changed;
                        if update.clicked && button.activate() {
                            button.set_label(if button.label() == "GUI test" {
                                "Clicked!"
                            } else {
                                "GUI test"
                            });
                            changed = true;
                        }
                    }
                }

                if left_is_down && !left_was_down {
                    let on_window_control = manager
                        .window_at(pointer_position)
                        .and_then(|id| manager.window(id))
                        .is_some_and(|window| {
                            gui::window_control_at(window.bounds(), pointer_position).is_some()
                                || (explorer_window == manager.active_id()
                                    && explorer.context_menu().is_some())
                                || (notepad_window == manager.active_id()
                                    && (gui::notepad_open_bounds(window.bounds())
                                        .contains(pointer_position)
                                        || gui::notepad_save_bounds(window.bounds())
                                            .contains(pointer_position)))
                        });
                    if !on_window_control
                        && button.state() != ButtonState::Pressed
                        && launcher.state() != ButtonState::Pressed
                    {
                        changed |= manager.begin_drag(pointer_position, 28).is_some();
                    }
                } else if left_is_down {
                    let old_bounds = manager
                        .active_id()
                        .and_then(|id| manager.window(id))
                        .map(|window| window.bounds());
                    if manager.drag_to(pointer_position) {
                        let new_bounds = manager
                            .active_id()
                            .and_then(|id| manager.window(id))
                            .map(|window| window.bounds());
                        if let (Some(old_bounds), Some(new_bounds)) = (old_bounds, new_bounds) {
                            if old_bounds != new_bounds {
                                drag_damage = Some(union_rects(old_bounds, new_bounds));
                                changed = true;
                            }
                        }
                    }
                } else if left_was_down {
                    manager.end_drag();
                }
            }
            left_was_down = left_is_down;

            if changed {
                if let Some(window) = button_window.and_then(|id| manager.window(id)) {
                    let bounds = window.bounds();
                    button.set_bounds(Rect::new(bounds.x + 18, bounds.y + 140, 128, 36));
                }
                redraw_scene(
                    &mut canvas,
                    &manager,
                    &start_menu,
                    &clock,
                    &launcher,
                    &button,
                    button_window,
                    notepad_window,
                    explorer_window,
                    calculator_window,
                    &notepad,
                    &explorer,
                    &calculator,
                    &mut cursor,
                    pointer_position,
                    pointer_visible,
                    drag_damage,
                );
            } else {
                cursor.hide(&mut canvas).unwrap();
                if pointer_visible {
                    cursor.show(&mut canvas, pointer_position).unwrap();
                }
            }
        }

        if clock_ticks == 100 {
            clock_ticks = 0;
            let updated_clock = current_clock();
            if updated_clock != clock {
                clock = updated_clock;
                redraw_scene(
                    &mut canvas,
                    &manager,
                    &start_menu,
                    &clock,
                    &launcher,
                    &button,
                    button_window,
                    notepad_window,
                    explorer_window,
                    calculator_window,
                    &notepad,
                    &explorer,
                    &calculator,
                    &mut cursor,
                    pointer_position,
                    pointer_visible,
                    None,
                );
            }
        } else {
            clock_ticks += 1;
        }

        boot::stall(core::time::Duration::from_millis(10));
    }
}
