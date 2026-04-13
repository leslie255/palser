# Palser

**Platform Abstraction Layer for SoftwarE Renderers**

Library that handle window creation, framebuffer presentation, and GUI event piping for software renderers.

## Example

```rs
//! ```cargo
//! [dependencies]
//! bytemuck = "1.25.0"
//! palser = { git = "https://github.com/leslie255/palser.git" }
//! ```

#[derive(Default)]
struct App {
    frame_buffer: Vec<u8>,
}

impl palser::ApplicationHandler for App {
    fn redraw_requested(
        &mut self,
        window_width: u32,
        window_height: u32,
        _dpi: f64,
    ) -> palser::FrameOutput<'_> {
        // Clear framebuffer.
        self.frame_buffer
            .resize(window_width as usize * window_height as usize * 4, 0);
        // Rendering.
        for rgba in self.frame_buffer.chunks_mut(4) {
            rgba.copy_from_slice(&[0xFF, 0x48, 0x48, 0xFF]);
        }
        // Present.
        palser::FrameOutput::new(
            window_width,
            window_height,
            palser::FramebufferFormat::Rgba8UnormSrgb,
            bytemuck::cast_slice(&self.frame_buffer[..]),
        )
        .window_title("Palser Example")
    }
}

fn main() {
    let mut app = App::default();
    palser::run_application(&mut app);
}
```

## LICENSE

This project is licensed under Apache License Version 2.0
