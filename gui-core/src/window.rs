use crate::{Point, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowError {
    CapacityReached,
    InvalidBounds,
    NotFound,
    NotNormal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Window {
    id: WindowId,
    title: &'static str,
    bounds: Rect,
    restore_bounds: Rect,
    state: WindowState,
}

impl Window {
    pub const fn id(&self) -> WindowId {
        self.id
    }

    pub const fn title(&self) -> &'static str {
        self.title
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub const fn state(&self) -> WindowState {
        self.state
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DragCapture {
    window: WindowId,
    offset_x: i32,
    offset_y: i32,
}

pub struct WindowManager<const N: usize> {
    windows: [Option<Window>; N],
    z_order: [Option<WindowId>; N],
    count: usize,
    next_id: u64,
    active: Option<WindowId>,
    work_area: Rect,
    drag: Option<DragCapture>,
}

impl<const N: usize> WindowManager<N> {
    pub const fn new(work_area: Rect) -> Self {
        Self {
            windows: [None; N],
            z_order: [None; N],
            count: 0,
            next_id: 1,
            active: None,
            work_area,
            drag: None,
        }
    }

    pub fn open(&mut self, title: &'static str, bounds: Rect) -> Result<WindowId, WindowError> {
        if bounds.width == 0 || bounds.height == 0 {
            return Err(WindowError::InvalidBounds);
        }

        let slot = self
            .windows
            .iter()
            .position(Option::is_none)
            .ok_or(WindowError::CapacityReached)?;
        let id = WindowId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.windows[slot] = Some(Window {
            id,
            title,
            bounds,
            restore_bounds: bounds,
            state: WindowState::Normal,
        });
        self.z_order[self.count] = Some(id);
        self.count += 1;
        self.active = Some(id);
        Ok(id)
    }

    pub fn close(&mut self, id: WindowId) -> Result<(), WindowError> {
        let slot = self.window_slot(id).ok_or(WindowError::NotFound)?;
        self.windows[slot] = None;
        self.remove_from_z_order(id);
        if self.active == Some(id) {
            self.active = self.top_visible_window(None);
        }
        if self.drag.is_some_and(|drag| drag.window == id) {
            self.drag = None;
        }
        Ok(())
    }

    pub fn focus(&mut self, id: WindowId) -> Result<bool, WindowError> {
        let window = self.window(id).ok_or(WindowError::NotFound)?;
        if window.state == WindowState::Minimized {
            return Err(WindowError::NotNormal);
        }

        let changed = self.active != Some(id);
        self.remove_from_z_order(id);
        self.z_order[self.count] = Some(id);
        self.count += 1;
        self.active = Some(id);
        Ok(changed)
    }

    pub fn focus_next(&mut self) -> Option<WindowId> {
        let mut visible = [None; N];
        let mut visible_count = 0;

        for id in self.z_order[..self.count].iter().filter_map(|entry| *entry) {
            if self
                .window(id)
                .is_some_and(|window| window.state != WindowState::Minimized)
            {
                visible[visible_count] = Some(id);
                visible_count += 1;
            }
        }

        if visible_count == 0 {
            return None;
        }

        let current_index = visible[..visible_count]
            .iter()
            .position(|id| *id == self.active)
            .unwrap_or(visible_count - 1);
        let next = visible[(current_index + 1) % visible_count]?;
        self.focus(next).ok()?;
        Some(next)
    }

    pub fn minimize(&mut self, id: WindowId) -> Result<bool, WindowError> {
        let window = self.window_mut(id).ok_or(WindowError::NotFound)?;
        if window.state == WindowState::Minimized {
            return Ok(false);
        }

        window.state = WindowState::Minimized;
        if self.active == Some(id) {
            self.active = self.top_visible_window(Some(id));
        }
        if self.drag.is_some_and(|drag| drag.window == id) {
            self.drag = None;
        }
        Ok(true)
    }

    pub fn maximize(&mut self, id: WindowId) -> Result<bool, WindowError> {
        let work_area = self.work_area;
        let window = self.window_mut(id).ok_or(WindowError::NotFound)?;
        if window.state == WindowState::Maximized {
            return Ok(false);
        }

        if window.state == WindowState::Normal {
            window.restore_bounds = window.bounds;
        }
        window.bounds = work_area;
        window.state = WindowState::Maximized;
        self.focus(id)?;
        self.drag = None;
        Ok(true)
    }

    pub fn restore(&mut self, id: WindowId) -> Result<bool, WindowError> {
        let window = self.window_mut(id).ok_or(WindowError::NotFound)?;
        if window.state == WindowState::Normal {
            return Ok(false);
        }

        if window.state == WindowState::Maximized {
            window.bounds = window.restore_bounds;
        }
        window.state = WindowState::Normal;
        self.focus(id)?;
        Ok(true)
    }

    pub fn move_to(&mut self, id: WindowId, x: i32, y: i32) -> Result<(), WindowError> {
        let window = self.window_mut(id).ok_or(WindowError::NotFound)?;
        if window.state != WindowState::Normal {
            return Err(WindowError::NotNormal);
        }

        window.bounds.x = x;
        window.bounds.y = y;
        Ok(())
    }

    pub fn resize_to(&mut self, id: WindowId, width: u32, height: u32) -> Result<(), WindowError> {
        if width == 0 || height == 0 {
            return Err(WindowError::InvalidBounds);
        }

        let window = self.window_mut(id).ok_or(WindowError::NotFound)?;
        if window.state != WindowState::Normal {
            return Err(WindowError::NotNormal);
        }

        window.bounds.width = width;
        window.bounds.height = height;
        Ok(())
    }

    pub fn begin_drag(&mut self, point: Point, title_bar_height: u32) -> Option<WindowId> {
        let id = self.window_at(point)?;
        self.focus(id).ok()?;
        let bounds = self.window(id)?.bounds;
        let title_bar_bottom = bounds.y.saturating_add(title_bar_height as i32);
        if self.window(id)?.state == WindowState::Normal && point.y < title_bar_bottom {
            self.drag = Some(DragCapture {
                window: id,
                offset_x: point.x - bounds.x,
                offset_y: point.y - bounds.y,
            });
        } else {
            self.drag = None;
        }
        Some(id)
    }

    pub fn drag_to(&mut self, point: Point) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        self.move_to(
            drag.window,
            point.x.saturating_sub(drag.offset_x),
            point.y.saturating_sub(drag.offset_y),
        )
        .is_ok()
    }

    pub fn end_drag(&mut self) {
        self.drag = None;
    }

    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.window_slot(id)
            .and_then(|slot| self.windows[slot].as_ref())
    }

    pub fn active_window(&self) -> Option<&Window> {
        self.active.and_then(|id| self.window(id))
    }

    pub const fn active_id(&self) -> Option<WindowId> {
        self.active
    }

    pub fn window_at(&self, point: Point) -> Option<WindowId> {
        self.z_order[..self.count]
            .iter()
            .rev()
            .filter_map(|entry| *entry)
            .find(|id| {
                self.window(*id).is_some_and(|window| {
                    window.state != WindowState::Minimized && window.bounds.contains(point)
                })
            })
    }

    pub fn windows_back_to_front(&self) -> impl Iterator<Item = &Window> {
        self.z_order[..self.count]
            .iter()
            .filter_map(|entry| entry.and_then(|id| self.window(id)))
    }

    fn window_slot(&self, id: WindowId) -> Option<usize> {
        self.windows
            .iter()
            .position(|window| window.is_some_and(|window| window.id == id))
    }

    fn window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        let slot = self.window_slot(id)?;
        self.windows[slot].as_mut()
    }

    fn remove_from_z_order(&mut self, id: WindowId) {
        let Some(slot) = self.z_order[..self.count]
            .iter()
            .position(|entry| *entry == Some(id))
        else {
            return;
        };

        for index in slot..self.count - 1 {
            self.z_order[index] = self.z_order[index + 1];
        }
        self.count -= 1;
        self.z_order[self.count] = None;
    }

    fn top_visible_window(&self, excluded: Option<WindowId>) -> Option<WindowId> {
        self.z_order[..self.count]
            .iter()
            .rev()
            .filter_map(|entry| *entry)
            .find(|id| {
                Some(*id) != excluded
                    && self
                        .window(*id)
                        .is_some_and(|window| window.state != WindowState::Minimized)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowError, WindowManager, WindowState};
    use crate::{Point, Rect};

    fn manager() -> WindowManager<3> {
        WindowManager::new(Rect::new(0, 0, 800, 600))
    }

    #[test]
    fn focus_raises_window_and_hit_testing_respects_z_order() {
        let mut manager = manager();
        let first = manager.open("First", Rect::new(10, 10, 200, 120)).unwrap();
        let second = manager.open("Second", Rect::new(30, 30, 200, 120)).unwrap();
        let overlap = Point { x: 40, y: 40 };

        assert_eq!(manager.window_at(overlap), Some(second));
        manager.focus(first).unwrap();
        assert_eq!(manager.active_id(), Some(first));
        assert_eq!(manager.window_at(overlap), Some(first));
    }

    #[test]
    fn maximize_and_restore_preserve_original_bounds() {
        let mut manager = manager();
        let bounds = Rect::new(40, 50, 320, 240);
        let id = manager.open("Window", bounds).unwrap();

        assert!(manager.maximize(id).unwrap());
        assert_eq!(manager.window(id).unwrap().state(), WindowState::Maximized);
        assert_eq!(
            manager.window(id).unwrap().bounds(),
            Rect::new(0, 0, 800, 600)
        );
        assert!(manager.restore(id).unwrap());
        assert_eq!(manager.window(id).unwrap().bounds(), bounds);
    }

    #[test]
    fn title_bar_drag_preserves_pointer_offset() {
        let mut manager = manager();
        let id = manager.open("Window", Rect::new(20, 30, 240, 180)).unwrap();

        assert_eq!(manager.begin_drag(Point { x: 28, y: 38 }, 24), Some(id));
        assert!(manager.drag_to(Point { x: 68, y: 78 }));
        assert_eq!(manager.window(id).unwrap().bounds().x, 60);
        assert_eq!(manager.window(id).unwrap().bounds().y, 70);
        manager.end_drag();
        assert!(!manager.drag_to(Point { x: 80, y: 90 }));
    }

    #[test]
    fn minimizing_active_window_selects_another_visible_window() {
        let mut manager = manager();
        let first = manager.open("First", Rect::new(10, 10, 200, 120)).unwrap();
        let second = manager.open("Second", Rect::new(30, 30, 200, 120)).unwrap();

        assert!(manager.minimize(second).unwrap());
        assert_eq!(manager.active_id(), Some(first));
        assert_eq!(manager.window_at(Point { x: 40, y: 40 }), Some(first));
        assert_eq!(manager.focus(second), Err(WindowError::NotNormal));
        manager.restore(second).unwrap();
        assert_eq!(manager.active_id(), Some(second));
    }

    #[test]
    fn focus_next_cycles_visible_windows_and_skips_minimized() {
        let mut manager = manager();
        let first = manager.open("First", Rect::new(10, 10, 200, 120)).unwrap();
        let second = manager.open("Second", Rect::new(30, 30, 200, 120)).unwrap();
        manager.minimize(second).unwrap();

        assert_eq!(manager.focus_next(), Some(first));
        assert_eq!(manager.focus_next(), Some(first));
    }
}
