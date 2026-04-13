#[derive(Default)]
struct App {
    framebuffer: Vec<u8>,
}

impl palser::ApplicationHandler for App {
    fn redraw_requested(
        &mut self,
        window_width: u32,
        window_height: u32,
        _dpi: f64,
    ) -> palser::FrameOutput<'_> {
        self.framebuffer
            .resize(window_width as usize * window_height as usize * 4, 0);

        for rgba in self.framebuffer.chunks_mut(4) {
            rgba.copy_from_slice(&[0xFF, 0x48, 0x48, 0xFF]);
        }

        palser::FrameOutput::new(
            window_width,
            window_height,
            palser::FramebufferFormat::Rgba8UnormSrgb,
            bytemuck::cast_slice(&self.framebuffer[..]),
        )
    }
}

fn main() {
    let mut app = App::default();
    palser::run_application(&mut app);
}
