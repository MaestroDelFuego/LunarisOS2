#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StartMenuState {
    open: bool,
    selected_item: usize,
    item_count: usize,
}

impl StartMenuState {
    pub const fn new(item_count: usize) -> Self {
        Self {
            open: false,
            selected_item: 0,
            item_count,
        }
    }

    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub const fn selected_item(&self) -> usize {
        self.selected_item
    }

    pub fn toggle(&mut self) -> bool {
        if self.item_count == 0 {
            return false;
        }

        self.open = !self.open;
        true
    }

    pub fn close(&mut self) -> bool {
        let was_open = self.open;
        self.open = false;
        was_open
    }

    pub fn move_selection(&mut self, direction: i8) -> bool {
        if !self.open || self.item_count < 2 {
            return false;
        }

        let previous = self.selected_item;
        self.selected_item = if direction < 0 {
            if previous == 0 {
                self.item_count - 1
            } else {
                previous - 1
            }
        } else if direction > 0 {
            (previous + 1) % self.item_count
        } else {
            previous
        };
        self.selected_item != previous
    }

    pub fn select_item(&mut self, index: usize) -> bool {
        if !self.open || index >= self.item_count || index == self.selected_item {
            return false;
        }
        self.selected_item = index;
        true
    }

    pub fn activate(&self) -> Option<usize> {
        (self.open && self.item_count > 0).then_some(self.selected_item)
    }
}

#[cfg(test)]
mod tests {
    use super::StartMenuState;

    #[test]
    fn start_menu_selects_and_activates_real_entries() {
        let mut menu = StartMenuState::new(2);

        assert_eq!(menu.activate(), None);
        assert!(menu.toggle());
        assert_eq!(menu.activate(), Some(0));
        assert!(menu.move_selection(1));
        assert_eq!(menu.selected_item(), 1);
        assert!(!menu.select_item(1));
        assert!(!menu.select_item(2));
        assert_eq!(menu.activate(), Some(1));
        assert!(menu.move_selection(1));
        assert_eq!(menu.selected_item(), 0);
        assert!(menu.close());
        assert!(!menu.is_open());
    }

    #[test]
    fn empty_start_menu_cannot_open_or_activate() {
        let mut menu = StartMenuState::new(0);

        assert!(!menu.toggle());
        assert_eq!(menu.activate(), None);
    }
}
