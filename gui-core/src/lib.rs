#![no_std]

mod calculator;
mod start_menu;
mod text_buffer;
mod window;

pub use calculator::Calculator;
pub use start_menu::StartMenuState;
pub use text_buffer::{cursor_at_visual, visual_cursor, TextBuffer, TextBufferError};
pub use window::{Window, WindowError, WindowId, WindowManager, WindowState};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(self, point: Point) -> bool {
        let Some(dx) = point.x.checked_sub(self.x) else {
            return false;
        };
        let Some(dy) = point.y.checked_sub(self.y) else {
            return false;
        };

        dx >= 0 && dy >= 0 && (dx as u32) < self.width && (dy as u32) < self.height
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ButtonUpdate {
    pub state_changed: bool,
    pub clicked: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Button {
    bounds: Rect,
    label: &'static str,
    state: ButtonState,
    enabled: bool,
    captured: bool,
    pointer_was_down: bool,
}

impl Button {
    pub const fn new(bounds: Rect, label: &'static str) -> Self {
        Self {
            bounds,
            label,
            state: ButtonState::Normal,
            enabled: true,
            captured: false,
            pointer_was_down: false,
        }
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.captured = false;
            self.pointer_was_down = false;
            self.state = if self.enabled {
                ButtonState::Normal
            } else {
                ButtonState::Disabled
            };
        }
    }

    pub const fn label(&self) -> &'static str {
        self.label
    }

    pub const fn state(&self) -> ButtonState {
        self.state
    }

    pub fn set_label(&mut self, label: &'static str) {
        self.label = label;
    }

    pub fn set_enabled(&mut self, enabled: bool) -> bool {
        if self.enabled == enabled {
            return false;
        }

        self.enabled = enabled;
        self.captured = false;
        self.state = if enabled {
            ButtonState::Normal
        } else {
            ButtonState::Disabled
        };
        true
    }

    pub const fn activate(&self) -> bool {
        self.enabled
    }

    pub fn update_pointer(&mut self, point: Point, left_down: bool) -> ButtonUpdate {
        let previous_state = self.state;
        let inside = self.bounds.contains(point);
        let mut clicked = false;

        if self.enabled {
            if left_down && !self.pointer_was_down && inside {
                self.captured = true;
            }

            if !left_down && self.pointer_was_down && self.captured {
                clicked = inside;
                self.captured = false;
            }

            self.state = if self.captured {
                ButtonState::Pressed
            } else if inside {
                ButtonState::Hovered
            } else {
                ButtonState::Normal
            };
        } else {
            self.state = ButtonState::Disabled;
            self.captured = false;
        }

        self.pointer_was_down = left_down;

        ButtonUpdate {
            state_changed: self.state != previous_state,
            clicked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Button, ButtonState, Point, Rect};

    #[test]
    fn rectangle_uses_half_open_bounds() {
        let rect = Rect::new(10, 20, 30, 15);

        assert!(rect.contains(Point { x: 10, y: 20 }));
        assert!(rect.contains(Point { x: 39, y: 34 }));
        assert!(!rect.contains(Point { x: 40, y: 34 }));
        assert!(!rect.contains(Point { x: 39, y: 35 }));
    }

    #[test]
    fn button_clicks_only_after_release_inside() {
        let mut button = Button::new(Rect::new(5, 5, 20, 12), "Open");
        let inside = Point { x: 8, y: 8 };
        let outside = Point { x: 40, y: 40 };

        assert!(!button.update_pointer(inside, true).clicked);
        assert_eq!(button.state(), ButtonState::Pressed);
        assert!(button.update_pointer(inside, false).clicked);
        assert_eq!(button.state(), ButtonState::Hovered);

        button.update_pointer(inside, true);
        assert!(!button.update_pointer(outside, false).clicked);
        assert_eq!(button.state(), ButtonState::Normal);
    }

    #[test]
    fn disabled_button_never_activates() {
        let mut button = Button::new(Rect::new(0, 0, 8, 8), "Disabled");

        assert!(button.activate());
        assert!(button.set_enabled(false));
        assert_eq!(button.state(), ButtonState::Disabled);
        assert!(!button.activate());
        button.update_pointer(Point { x: 2, y: 2 }, true);
        assert!(!button.update_pointer(Point { x: 2, y: 2 }, false).clicked);
    }

    #[test]
    fn moving_button_bounds_moves_its_hit_area() {
        let mut button = Button::new(Rect::new(0, 0, 8, 8), "Move");
        button.set_bounds(Rect::new(20, 30, 8, 8));

        assert!(
            !button
                .update_pointer(Point { x: 2, y: 2 }, true)
                .state_changed
        );
        assert!(!button.update_pointer(Point { x: 22, y: 32 }, false).clicked);
        assert_eq!(button.state(), ButtonState::Hovered);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}
