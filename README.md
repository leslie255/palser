# Palser

**Platform Abstraction Layer for SoftwarE Renderers**

Library that handle window creation, framebuffer presentation, and GUI event piping for software renderers.

## Example

```rs
#[derive(Default)]
struct App {
    framebuffer: Vec<u8>,
}

impl palser::ApplicationHandler for App {
    fn redraw_requested(&mut self, width: u32, height: u32, _dpi: f64) -> palser::FrameOutput<'_> {
        // Resize framebuffer (if needed).
        self.framebuffer
            .resize(width as usize * height as usize * 4, 0);

        // Render a checkerboard pattern.
        for y in 0..height {
            for x in 0..width {
                let color = match x / 32 + y / 32 {
                    i if i.is_multiple_of(2) => [0x80, 0xFF, 0xFF, 0xFF],
                    _ => [0xFF, 0x80, 0x80, 0xFF],
                };
                let offset = (y as usize * width as usize + x as usize) * 4;
                self.framebuffer[offset..offset + 4].copy_from_slice(&color);
            }
        }

        // Submit for present.
        palser::FrameOutput::new(
            width,
            height,
            palser::FramebufferFormat::Rgba8UnormSrgb,
            &self.framebuffer[..],
        )
    }
}

fn main() {
    let mut app = App::default();
    palser::run_application(&mut app);
}
```

Result:

![Screenshot 2026-04-13 at 08 54 32](https://github.com/user-attachments/assets/4615bf67-e44c-4b3c-a03a-5954e710d6be)


For more examples, see `examples/` directory.

## LICENSE
This project is licensed under Apache License Version 2.0
