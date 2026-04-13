use bytemuck::{Pod, Zeroable};

const MAX_FRAME_WIDTH: u32 = 960;
const MAX_FRAME_HEIGHT: u32 = 720;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
#[repr(align(4))]
pub struct RgbaU8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaU8 {
    pub const fn hex(u: u32) -> Self {
        let [r, g, b, a] = u.to_be_bytes();
        Self { r, g, b, a }
    }
}

#[derive(Default)]
struct App {
    frame_width: u32,
    frame_height: u32,
    frame_buffer: Vec<RgbaU8>,
    cursor_locked: bool,
    cursor_hidden: bool,
    input_state: palser::utils::InputState,
}

impl App {
    #[inline(always)]
    fn assert_framebuffer_valid(&self) {
        let n_pixels = self.frame_width as usize * self.frame_height as usize;
        assert_eq!(self.frame_buffer.len(), n_pixels);
        assert!(self.frame_width != 0);
        assert!(self.frame_height != 0);
    }

    fn fill_rect(&mut self, x_min: u32, y_min: u32, width: u32, height: u32, color: RgbaU8) {
        self.assert_framebuffer_valid();
        let x_max = (x_min + width).min(self.frame_width - 1);
        let y_max = (y_min + height).min(self.frame_height - 1);

        for x in x_min..x_max {
            for y in y_min..y_max {
                let i_pixel = y as usize * self.frame_width as usize + x as usize;
                // Safety:
                // - asserted `frame_buffer.len()` size is well-formed against
                //   `frame_{width|height}`
                // - `{x|y}_{min|max}` are clamped range (hence `x`, `y`, and `i_pixel` are in
                //   range too)
                unsafe { *self.frame_buffer.get_unchecked_mut(i_pixel) = color };
            }
        }
    }

    fn fill_circle(&mut self, center_x: i64, center_y: i64, radius: u32, color: RgbaU8) {
        self.assert_framebuffer_valid();

        let x_min = center_x as isize - radius as isize;
        let x_max = center_x as isize + radius as isize;
        let y_min = center_y as isize - radius as isize;
        let y_max = center_y as isize + radius as isize;

        for x in x_min..x_max {
            for y in y_min..y_max {
                if !(0..self.frame_width as isize).contains(&x)
                    || !(0..self.frame_height as isize).contains(&y)
                {
                    continue;
                }

                let dx = x as f32 - center_x as f32;
                let dy = y as f32 - center_y as f32;
                let r2 = dx.powi(2) + dy.powi(2);
                if r2 > (radius as f32).powi(2) {
                    continue;
                }

                let i_pixel = y as usize * self.frame_width as usize + x as usize;
                // Safety:
                // - asserted `frame_buffer.len()` size is well-formed against
                //   `frame_{width|height}`
                // - `x` and `y` are made sure to be in range earlier (therefore `i_pixel` is in
                //   range too)
                unsafe { *self.frame_buffer.get_unchecked_mut(i_pixel) = color };
            }
        }
    }
}

impl palser::ApplicationHandler for App {
    fn redraw_requested(
        &mut self,
        window_width: u32,
        window_height: u32,
        dpi: f64,
    ) -> palser::FrameOutput<'_> {
        (self.frame_width, self.frame_height) = palser::utils::clamp_frame_size(
            dpi,
            window_width,
            window_height,
            MAX_FRAME_WIDTH,
            MAX_FRAME_HEIGHT,
        );

        self.frame_buffer.resize(
            self.frame_width as usize * self.frame_height as usize,
            RgbaU8::zeroed(),
        );
        self.frame_buffer.fill(RgbaU8::hex(0x181818FF));

        self.fill_rect(10, 10, 100, 100, RgbaU8::hex(0xFFFFFFFF));
        if let Some((x, y)) = self.input_state.cursor_position() {
            self.fill_circle(x as i64, y as i64, 24, RgbaU8::hex(0xFF4848FF));
        }

        palser::FrameOutput::new(
            self.frame_width,
            self.frame_height,
            palser::FramebufferFormat::Rgba8UnormSrgb,
            bytemuck::cast_slice(&self.frame_buffer),
        )
        .hide_cursor(self.cursor_hidden)
        .lock_cursor(self.cursor_locked)
    }

    fn key_pressed(&mut self, key_code: palser::KeyCode, is_repeat: bool) {
        println!("key pressed: {key_code:?}, repeat: {is_repeat}");
        self.input_state.notify_key_pressed(key_code);

        match key_code {
            palser::KeyCode::KeyL => self.cursor_locked = !self.cursor_locked,
            palser::KeyCode::KeyH => self.cursor_hidden = !self.cursor_hidden,
            palser::KeyCode::KeyI if self.input_state.alt_down() => {
                dbg!(&self.input_state);
            }
            _ => (),
        }
    }

    fn key_released(&mut self, key_code: palser::KeyCode) {
        println!("key released: {key_code:?}");
        self.input_state.notify_key_released(key_code);
    }

    fn mouse_moved_to_position(&mut self, x: f64, y: f64) {
        self.input_state.notify_mouse_moved_to_position(x, y);
    }

    fn mouse_button_pressed(&mut self, button: palser::MouseButton) {
        self.input_state.notify_mouse_button_pressed(button)
    }

    fn mouse_button_released(&mut self, button: palser::MouseButton) {
        self.input_state.notify_mouse_button_released(button)
    }

    fn cursor_in_window(&mut self, in_window: bool) {
        self.input_state.notify_cursor_in_window(in_window);
    }
}

fn main() {
    let mut app = App::default();
    palser::run_application(&mut app);
}
