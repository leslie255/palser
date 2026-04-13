use crate::*;

pub fn clamp_frame_size(
    dpi: f64,
    window_width: u32,
    window_height: u32,
    max_width: u32,
    max_height: u32,
) -> (u32, u32) {
    let window_width = window_width as f64 * dpi;
    let window_height = window_height as f64 * dpi;
    let scale = (max_width as f64 / window_width)
        .min(max_height as f64 / window_height)
        .min(1.0);
    (
        (window_width * scale).round() as u32,
        (window_height * scale).round() as u32,
    )
}

#[derive(Clone)]
pub struct InputState {
    key_states: Box<[bool; 256]>,
    cursor_position_x: f64,
    cursor_position_y: f64,
    cursor_in_window: bool,
    mouse_left_state: bool,
    mouse_right_state: bool,
    mouse_middle_state: bool,
}

impl std::fmt::Debug for InputState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key_states = std::fmt::from_fn(|f| {
            f.debug_list()
                .entries(
                    self.key_states
                        .iter()
                        .enumerate()
                        .filter_map(|(u, &state)| {
                            if state && let Some(key) = KeyCode::from_u8(u as u8) {
                                Some(key)
                            } else {
                                None
                            }
                        }),
                )
                .finish()
        });
        f.debug_struct("InputState")
            .field("key_states(down)", &key_states)
            .field("cursor_position_x", &self.cursor_position_x)
            .field("cursor_position_y", &self.cursor_position_y)
            .field("cursor_in_window", &self.cursor_in_window)
            .field("mouse_left_state", &self.mouse_left_state)
            .field("mouse_right_state", &self.mouse_right_state)
            .field("mouse_middle_state", &self.mouse_middle_state)
            .finish()
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            key_states: Box::from([false; 256]),
            cursor_position_x: 0.0,
            cursor_position_y: 0.0,
            cursor_in_window: false,
            mouse_left_state: false,
            mouse_right_state: false,
            mouse_middle_state: false,
        }
    }

    pub fn reset(&mut self) {
        self.key_states.fill(false);
        self.cursor_position_x = 0.0;
        self.cursor_position_y = 0.0;
        self.cursor_in_window = false;
        self.mouse_left_state = false;
        self.mouse_right_state = false;
        self.mouse_middle_state = false;
    }

    pub fn notify_key_pressed(&mut self, key_code: KeyCode) {
        self.key_states[key_code as u8 as usize] = true;
    }

    pub fn notify_key_released(&mut self, key_code: KeyCode) {
        self.key_states[key_code as u8 as usize] = false;
    }

    pub fn notify_mouse_moved_to_position(&mut self, x: f64, y: f64) {
        self.cursor_position_x = x;
        self.cursor_position_y = y;
    }

    pub fn notify_mouse_button_pressed(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => self.mouse_left_state = true,
            MouseButton::Right => self.mouse_right_state = true,
            MouseButton::Middle => self.mouse_middle_state = true,
        }
    }

    pub fn notify_mouse_button_released(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => self.mouse_left_state = false,
            MouseButton::Right => self.mouse_right_state = false,
            MouseButton::Middle => self.mouse_middle_state = false,
        }
    }

    pub fn notify_cursor_in_window(&mut self, in_window: bool) {
        self.cursor_in_window = in_window;
    }

    /// Requires implemented:
    /// - `notify_key_pressed`
    /// - `notify_key_released`
    pub fn key_down(&self, key_code: KeyCode) -> bool {
        self.key_states[key_code as u8 as usize]
    }

    /// Returns `None` if cursor is not in window.
    ///
    /// Requires implemented:
    /// - `notify_cursor_in_window`
    /// - `notify_mouse_moved_to_position`
    pub fn cursor_position(&self) -> Option<(f64, f64)> {
        if self.cursor_in_window {
            Some((self.cursor_position_x, self.cursor_position_y))
        } else {
            None
        }
    }

    /// Requires implemented:
    /// - `notify_mouse_button_pressed`
    /// - `notify_mouse_button_released`
    pub fn mouse_button_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_left_state,
            MouseButton::Right => self.mouse_right_state,
            MouseButton::Middle => self.mouse_middle_state,
        }
    }

    /// Super for macOS, control for other OS's.
    ///
    /// Requires implemented:
    /// - `notify_key_pressed`
    /// - `notify_key_released`
    pub fn command_down(&self) -> bool {
        if cfg!(target_os = "macos") {
            self.command_down()
        } else {
            self.control_down()
        }
    }

    /// Either of the left/right super key is down.
    ///
    /// Requires implemented:
    /// - `notify_key_pressed`
    /// - `notify_key_released`
    pub fn super_down(&self) -> bool {
        self.key_down(KeyCode::SuperLeft) || self.key_down(KeyCode::SuperRight)
    }

    /// Either of the left/right control key is down.
    ///
    /// Requires implemented:
    /// - `notify_key_pressed`
    /// - `notify_key_released`
    pub fn control_down(&self) -> bool {
        self.key_down(KeyCode::ControlLeft) || self.key_down(KeyCode::ControlRight)
    }

    /// Either of the left/right alt key is down.
    ///
    /// Requires implemented:
    /// - `notify_key_pressed`
    /// - `notify_key_released`
    pub fn alt_down(&self) -> bool {
        self.key_down(KeyCode::AltLeft) || self.key_down(KeyCode::AltRight)
    }

    /// Either of the left/right shift key is down.
    ///
    /// Requires implemented:
    /// - `notify_key_pressed`
    /// - `notify_key_released`
    pub fn shift_down(&self) -> bool {
        self.key_down(KeyCode::ShiftLeft) || self.key_down(KeyCode::ShiftRight)
    }
}
