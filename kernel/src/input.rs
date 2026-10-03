use core::ptr::NonNull;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, SearchType};
use uefi::proto::console::pointer::{AbsolutePointer, Pointer};
use uefi::proto::console::text::{Key, ScanCode};
use uefi::system;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowAction {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    NextWindow,
    ToggleMaximize,
    Close,
    OpenTestWindow,
    ActivateControl,
    ToggleStartMenu,
}

pub fn read_key() -> Option<Key> {
    system::with_stdin(|stdin| stdin.read_key().ok().flatten())
}

pub fn window_action(key: Key) -> Option<WindowAction> {
    match key {
        Key::Special(ScanCode::LEFT) => Some(WindowAction::MoveLeft),
        Key::Special(ScanCode::RIGHT) => Some(WindowAction::MoveRight),
        Key::Special(ScanCode::UP) => Some(WindowAction::MoveUp),
        Key::Special(ScanCode::DOWN) => Some(WindowAction::MoveDown),
        Key::Special(ScanCode::ESCAPE) => Some(WindowAction::Close),
        Key::Special(ScanCode::FUNCTION_1) => Some(WindowAction::OpenTestWindow),
        Key::Special(ScanCode::FUNCTION_2) => Some(WindowAction::ToggleStartMenu),
        Key::Printable(character) if character == '\t' => Some(WindowAction::NextWindow),
        Key::Printable(character) if character == ' ' => Some(WindowAction::ActivateControl),
        Key::Printable(character) if character == '\r' || character == '\n' => {
            Some(WindowAction::ToggleMaximize)
        }
        _ => None,
    }
}

pub struct MouseInput {
    pub movement_x: i32,
    pub movement_y: i32,
    pub position: Option<Point>,
    pub left_down: bool,
    pub right_down: bool,
}

#[derive(Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

const MAX_POINTER_DEVICES: usize = 8;
const RELATIVE_MOUSE_SENSITIVITY: i32 = 4;

pub struct MouseDevice {
    relative: [Option<NonNull<Pointer>>; MAX_POINTER_DEVICES],
    absolute: [Option<NonNull<AbsolutePointer>>; MAX_POINTER_DEVICES],
    ps2: Option<Ps2Mouse>,
}

impl MouseDevice {
    pub fn open() -> Option<Self> {
        let mut relative = core::array::from_fn(|_| None);
        let mut absolute = core::array::from_fn(|_| None);
        let mut relative_count = 0;
        let mut absolute_count = 0;

        if let Ok(handles) = boot::locate_handle_buffer(SearchType::from_proto::<Pointer>()) {
            for handle in handles.iter() {
                if let Ok(pointer) =
                    open_pointer_protocol::<Pointer>(*handle, |pointer| pointer.reset(false))
                {
                    if relative_count < MAX_POINTER_DEVICES {
                        relative[relative_count] = Some(pointer);
                        relative_count += 1;
                    } else {
                        uefi::println!("Additional relative pointer ignored (limit reached).");
                    }
                }
            }
        }

        if let Ok(handles) = boot::locate_handle_buffer(SearchType::from_proto::<AbsolutePointer>())
        {
            for handle in handles.iter() {
                if let Ok(pointer) = open_pointer_protocol::<AbsolutePointer>(*handle, |pointer| {
                    pointer.reset(false)
                }) {
                    if absolute_count < MAX_POINTER_DEVICES {
                        absolute[absolute_count] = Some(pointer);
                        absolute_count += 1;
                    } else {
                        uefi::println!("Additional absolute pointer ignored (limit reached).");
                    }
                }
            }
        }

        let ps2 = match Ps2Mouse::open() {
            Ok(mouse) => {
                uefi::println!("PS/2 mouse fallback initialized.");
                Some(mouse)
            }
            Err(()) => {
                uefi::println!("PS/2 mouse fallback unavailable.");
                None
            }
        };

        (relative_count > 0 || absolute_count > 0 || ps2.is_some()).then_some(Self {
            relative,
            absolute,
            ps2,
        })
    }

    pub fn device_counts(&self) -> (usize, usize, bool) {
        (
            self.relative.iter().flatten().count(),
            self.absolute.iter().flatten().count(),
            self.ps2.is_some(),
        )
    }

    pub fn read(&mut self, width: usize, height: usize) -> Option<MouseInput> {
        for device in &mut self.relative {
            let Some(pointer) = *device else {
                continue;
            };
            // The UEFI input driver owns initialization; the kernel only polls it.
            let pointer = unsafe { pointer.as_ptr().as_mut().unwrap() };
            match pointer.read_state() {
                Ok(Some(state)) => {
                    return Some(MouseInput {
                        movement_x: scale_relative_delta(state.relative_movement_x),
                        movement_y: scale_relative_delta(state.relative_movement_y),
                        position: None,
                        left_down: state.left_button.0 != 0,
                        right_down: state.right_button.0 != 0,
                    });
                }
                Ok(None) => {}
                Err(error) => {
                    uefi::println!("Mouse input read failed: {:?}", error);
                    *device = None;
                }
            }
        }

        for device in &mut self.absolute {
            let Some(pointer) = *device else {
                continue;
            };
            // The UEFI input driver owns initialization; the kernel only polls it.
            let pointer = unsafe { pointer.as_ptr().as_mut().unwrap() };
            match pointer.read_state() {
                Ok(Some(state)) => {
                    let mode = pointer.mode();
                    return Some(MouseInput {
                        movement_x: 0,
                        movement_y: 0,
                        position: Some(Point {
                            x: scale_absolute(
                                state.current_x,
                                mode.absolute_min_x,
                                mode.absolute_max_x,
                                width,
                            ),
                            y: scale_absolute(
                                state.current_y,
                                mode.absolute_min_y,
                                mode.absolute_max_y,
                                height,
                            ),
                        }),
                        left_down: state.active_buttons & 1 != 0,
                        right_down: state.active_buttons & 2 != 0,
                    });
                }
                Ok(None) => {}
                Err(error) => {
                    uefi::println!("Mouse input read failed: {:?}", error);
                    *device = None;
                }
            }
        }

        self.ps2.as_mut().and_then(Ps2Mouse::read)
    }
}

const PS2_STATUS_PORT: u16 = 0x64;
const PS2_DATA_PORT: u16 = 0x60;
const PS2_WAIT_LIMIT: usize = 100_000;

struct Ps2Mouse {
    packet: [u8; 3],
    packet_length: usize,
}

impl Ps2Mouse {
    fn open() -> Result<Self, ()> {
        let mouse = Self {
            packet: [0; 3],
            packet_length: 0,
        };

        mouse.controller_command(0xa8)?;
        mouse.controller_command(0x20)?;
        let mut config = mouse.controller_read()?;
        config &= !0x22;
        mouse.controller_command(0x60)?;
        mouse.controller_write(config)?;
        mouse.auxiliary_write(0xf6)?;
        mouse.auxiliary_write(0xf4)?;

        Ok(mouse)
    }

    fn read(&mut self) -> Option<MouseInput> {
        let mut result = None;
        for _ in 0..16 {
            let status = unsafe { port_read(PS2_STATUS_PORT) };
            if status & 0x21 != 0x21 {
                break;
            }

            let byte = unsafe { port_read(PS2_DATA_PORT) };
            if self.packet_length == 0 {
                if byte & 0x08 == 0 {
                    continue;
                }
                self.packet[0] = byte;
                self.packet_length = 1;
            } else {
                self.packet[self.packet_length] = byte;
                self.packet_length += 1;
                if self.packet_length == self.packet.len() {
                    result = self.decode_packet();
                    self.packet_length = 0;
                }
            }
        }
        result
    }

    fn decode_packet(&self) -> Option<MouseInput> {
        let flags = self.packet[0];
        if flags & 0xc0 != 0 {
            return None;
        }

        Some(MouseInput {
            movement_x: scale_relative_delta(i32::from(self.packet[1] as i8)),
            movement_y: scale_relative_delta(-i32::from(self.packet[2] as i8)),
            position: None,
            left_down: flags & 1 != 0,
            right_down: flags & 2 != 0,
        })
    }

    fn controller_command(&self, command: u8) -> Result<(), ()> {
        wait_for_input_clear()?;
        unsafe { port_write(PS2_STATUS_PORT, command) };
        Ok(())
    }

    fn controller_write(&self, value: u8) -> Result<(), ()> {
        wait_for_input_clear()?;
        unsafe { port_write(PS2_DATA_PORT, value) };
        Ok(())
    }

    fn controller_read(&self) -> Result<u8, ()> {
        for _ in 0..PS2_WAIT_LIMIT {
            let status = unsafe { port_read(PS2_STATUS_PORT) };
            if status & 1 != 0 && status & 0x20 == 0 {
                return Ok(unsafe { port_read(PS2_DATA_PORT) });
            }
            core::hint::spin_loop();
        }
        Err(())
    }

    fn auxiliary_write(&self, value: u8) -> Result<(), ()> {
        for _ in 0..3 {
            self.controller_command(0xd4)?;
            self.controller_write(value)?;
            let response = self.controller_read_auxiliary()?;
            if response == 0xfa {
                return Ok(());
            }
            if response != 0xfe {
                return Err(());
            }
        }
        Err(())
    }

    fn controller_read_auxiliary(&self) -> Result<u8, ()> {
        for _ in 0..PS2_WAIT_LIMIT {
            let status = unsafe { port_read(PS2_STATUS_PORT) };
            if status & 0x21 == 0x21 {
                return Ok(unsafe { port_read(PS2_DATA_PORT) });
            }
            core::hint::spin_loop();
        }
        Err(())
    }
}

fn wait_for_input_clear() -> Result<(), ()> {
    for _ in 0..PS2_WAIT_LIMIT {
        if unsafe { port_read(PS2_STATUS_PORT) } & 0x02 == 0 {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err(())
}

unsafe fn port_read(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

unsafe fn port_write(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

fn scale_relative_delta(delta: i32) -> i32 {
    delta.saturating_mul(RELATIVE_MOUSE_SENSITIVITY)
}

#[cfg(test)]
mod tests {
    use super::{scale_relative_delta, RELATIVE_MOUSE_SENSITIVITY};

    #[test]
    fn relative_motion_uses_configured_sensitivity() {
        assert_eq!(scale_relative_delta(1), RELATIVE_MOUSE_SENSITIVITY);
        assert_eq!(scale_relative_delta(-3), -3 * RELATIVE_MOUSE_SENSITIVITY);
    }

    #[test]
    fn relative_motion_saturates_instead_of_overflowing() {
        assert_eq!(scale_relative_delta(i32::MAX), i32::MAX);
        assert_eq!(scale_relative_delta(i32::MIN), i32::MIN);
    }
}

fn open_pointer_protocol<P: uefi::proto::ProtocolPointer + ?Sized>(
    handle: uefi::Handle,
    reset: fn(&mut P) -> uefi::Result,
) -> uefi::Result<NonNull<P>> {
    // Exclusive access can stop the firmware's USB HID driver, leaving its
    // pointer protocol installed but unable to receive further mouse reports.
    let mut protocol = unsafe {
        boot::open_protocol::<P>(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )?
    };
    let interface = NonNull::from(&mut *protocol);
    if let Err(error) = reset(&mut *protocol) {
        uefi::println!("Pointer device reset failed: {:?}", error);
    }

    // GET_PROTOCOL has no matching CloseProtocol operation. Keep the interface
    // valid for this boot-services application and avoid ScopedProtocol's drop.
    core::mem::forget(protocol);
    Ok(interface)
}

fn scale_absolute(value: u64, minimum: u64, maximum: u64, pixels: usize) -> i32 {
    if maximum <= minimum || pixels <= 1 {
        return 0;
    }

    let position = value.clamp(minimum, maximum) - minimum;
    let range = maximum - minimum;
    ((u128::from(position) * (pixels - 1) as u128 / u128::from(range)) as usize) as i32
}
