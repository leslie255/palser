//! # PAL: Platform Abstraction Layer
//!
//! Responsible for window creation, frame buffer presentation and piping GUI events to the
//! software rendered content.

mod app_wrapper;
mod key_code;
pub mod utils;

pub use key_code::{KeyCode, MouseButton};

use std::path::PathBuf;

pub fn run_application(app: &mut dyn ApplicationHandler) {
    use app_wrapper::*;
    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    let mut wrapper_app = ApplicationWrapper::new(app);
    event_loop.run_app(&mut wrapper_app).unwrap();
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
#[repr(C)]
pub enum FramebufferFormat {
    Rgba8UnormSrgb = 1,
    Bgra8UnormSrgb,
    Rgba8UnormLinear,
    Bgra8UnormLinaer,
    Rgb32FloatLinear,
    Rgba32FloatLinear,
}

impl FramebufferFormat {
    /// Number of bytes per pixel of this format.
    pub const fn pixel_depth(self) -> usize {
        match self {
            FramebufferFormat::Rgba8UnormSrgb => 4,
            FramebufferFormat::Bgra8UnormSrgb => 4,
            FramebufferFormat::Rgba8UnormLinear => 4,
            FramebufferFormat::Bgra8UnormLinaer => 4,
            FramebufferFormat::Rgb32FloatLinear => 12,
            FramebufferFormat::Rgba32FloatLinear => 16,
        }
    }

    pub const fn is_srgb(self) -> bool {
        match self {
            FramebufferFormat::Rgba8UnormSrgb => true,
            FramebufferFormat::Bgra8UnormSrgb => true,
            FramebufferFormat::Rgba8UnormLinear => false,
            FramebufferFormat::Bgra8UnormLinaer => false,
            FramebufferFormat::Rgb32FloatLinear => false,
            FramebufferFormat::Rgba32FloatLinear => false,
        }
    }

    pub(crate) const fn to_wgpu_format(self) -> wgpu::TextureFormat {
        match self {
            FramebufferFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            FramebufferFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
            FramebufferFormat::Rgba8UnormLinear => wgpu::TextureFormat::Rgba8Unorm,
            FramebufferFormat::Bgra8UnormLinaer => wgpu::TextureFormat::Bgra8Unorm,
            FramebufferFormat::Rgb32FloatLinear => wgpu::TextureFormat::Rgba32Float,
            FramebufferFormat::Rgba32FloatLinear => wgpu::TextureFormat::Rgba32Float,
        }
    }
}

#[derive(Clone, Copy)]
#[non_exhaustive]
#[repr(C)]
pub struct FrameOutput<'a> {
    /// Width of the display.
    /// Note that this doesn't have to equal to the window' dimensions, content will be resampled
    /// to fit the dimension of the window size.
    pub framebuffer_width: u32,

    /// Height of the display.
    /// Note that this doesn't have to equal to the window's dimensions, content will be resampled
    /// to fit the dimension of the window size.
    pub framebuffer_height: u32,

    pub framebuffer_format: FramebufferFormat,

    /// Must be of length WIDTH*HEIGHT*DEPTH, where `WIDTH` and `HEIGHT` is the dimension of the
    /// framebuffer, `DEPTH` is `framebuffer_format.pixel_depth()`.
    ///
    /// Note that even for float framebuffer formats, the alignment of the pointer can still be 1!
    pub framebuffer_data: &'a [u8],

    /// Title of the window.
    pub window_title: &'a str,

    /// Use this field to request a window resize.
    pub request_window_resize: Option<[u32; 2]>,

    /// Use this field to request locking the cursor in a position.
    pub lock_cursor: bool,

    /// Use this field to request hiding the cursor.
    pub hide_cursor: bool,

    /// Use this field to request exit.
    pub request_exit: bool,
}

impl<'a> std::fmt::Debug for FrameOutput<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedrawResponse")
            .field("framebuffer_width", &self.framebuffer_width)
            .field("framebuffer_height", &self.framebuffer_height)
            .field("framebuffer_data", &self.framebuffer_data.as_ptr_range())
            .field("request_window_resize", &self.request_window_resize)
            .field("grab_cursor", &self.lock_cursor)
            .field("hide_cursor", &self.hide_cursor)
            .field("exiting", &self.request_exit)
            .finish()
    }
}

impl<'a> FrameOutput<'a> {
    pub fn new(
        framebuffer_width: u32,
        framebuffer_height: u32,
        framebuffer_format: FramebufferFormat,
        framebuffer_data: &'a [u8],
    ) -> Self {
        Self {
            framebuffer_width,
            framebuffer_height,
            framebuffer_format,
            framebuffer_data,
            window_title: DEFAULT_WINDOW_TITLE,
            request_window_resize: None,
            lock_cursor: false,
            hide_cursor: false,
            request_exit: false,
        }
    }

    /// Sets `window_title` field.
    pub fn window_title(self, window_title: &'a str) -> Self {
        Self {
            window_title,
            ..self
        }
    }

    /// Sets `request_window_resize` field.
    pub fn request_window_resize(self, width: u32, height: u32) -> Self {
        Self {
            request_window_resize: Some([width, height]),
            ..self
        }
    }

    /// Sets `lock_cursor` field.
    pub fn lock_cursor(self, lock_cursor: bool) -> Self {
        Self {
            lock_cursor,
            ..self
        }
    }

    /// Sets `hide_cursor` field.
    pub fn hide_cursor(self, hide_cursor: bool) -> Self {
        Self {
            hide_cursor,
            ..self
        }
    }

    /// Sets `request_exit` field.
    pub fn request_exit(self, request_exit: bool) -> Self {
        Self {
            request_exit,
            ..self
        }
    }
}

#[allow(unused_variables)]
pub trait ApplicationHandler {
    /// OS has requested a redraw of the window's content.
    ///
    /// # Arguments
    ///
    /// * window_width - the physical width of the window ("physical" meaning pre-DPI scaling)
    /// * window_height - the physical height of the window ("physical" meaning pre-DPI scaling)
    /// * dpi - the DPI of the current display
    ///
    /// # Notes
    ///
    /// Returned `FrameOutput` must have a "well-formed" framebuffer, which includes:
    /// - `frame_width` and `frame_height` must be both non-zero
    /// - `framebuffer_data`'s length must equal `frame_width * frame_height * 4`
    fn redraw_requested(
        &mut self,
        window_width: u32,
        window_height: u32,
        dpi: f64,
    ) -> FrameOutput<'_>;

    /// Notifies that OS has requested the application to exit.
    ///
    /// Return `true` for immediate exit.
    /// Return `false` if wanted to delay the exit (e.g. prompt file saving dialogue), an exit can
    /// be requested later on a `redraw_event`.
    fn exit_requested(&mut self) -> bool {
        true
    }

    /// Notifies a key being pressed.
    fn key_pressed(&mut self, key_code: KeyCode, is_repeat: bool) {}

    fn window_resized(&mut self, window_width: u32, window_height: u32, dpi: f64) {}

    /// Notifies a key being released.
    fn key_released(&mut self, key_code: KeyCode) {}

    /// Notifies a mouse button being pressed.
    fn mouse_button_pressed(&mut self, button: MouseButton) {}

    /// Notifies a mouse button being released.
    fn mouse_button_released(&mut self, button: MouseButton) {}

    /// Notifies the window being focused/unfocused.
    fn focus_changed(&mut self, is_focused: bool) {}

    /// Notifies a cursor movement in terms of the movement delta.
    ///
    /// # Notes
    /// - a cursor movement will trigger both `mouse_moved_to_position` and `mouse_moved_by_delta`
    /// - this function is called with physical (no scaling) `x` and `y` deltas, whereas
    ///   `mouse_moved_to_position` is called with logical `x` and `y` deltas, which is scaled to
    ///   give the location of the cursor in terms of the framebuffer's size, not the window size
    ///
    /// When cursor is grabbed, a cursor movement will still trigger this function with a
    /// "hypothetical" movement delta, even if the actual cursor position is not changed.
    fn mouse_moved_by_delta(&mut self, x: f64, y: f64) {}

    /// Notifies a cursor movement in terms of the new position.
    ///
    /// # Notes
    /// - a mouse movement will trigger both `mouse_moved_to_position` and `mouse_moved_by_delta`
    /// - this function is called with logical `x` and `y` deltas, which is scaled to give the
    ///   location of the cursor in terms of the framebuffer's size, not the window size; on the
    ///   other hand, `mouse_moved_by_delta` is called with the physical (no scaling) movement
    ///   delta
    fn mouse_moved_to_position(&mut self, x: f64, y: f64) {}

    /// Notifies that the cursor entered/left the window.
    fn cursor_in_window(&mut self, in_window: bool) {}

    /// Notifies that a file hovering started.
    fn hovered_file(&mut self, path: PathBuf) {}

    /// Notifies that a file hovering canceled.
    fn hovered_file_canceled(&mut self) {}

    /// Notifies that a file dropped.
    fn dropped_file(&mut self, path: PathBuf) {}
}

const DEFAULT_WINDOW_TITLE: &str = "Window";
