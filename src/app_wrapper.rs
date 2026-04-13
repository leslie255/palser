use std::sync::Arc;

use pollster::FutureExt as _;

use crate::{key_code::MouseButton, *};

const fn expected_framebuffer_pixels(width: u32, height: u32) -> usize {
    width as usize * height as usize
}

const fn expected_framebuffer_size(width: u32, height: u32, format: FramebufferFormat) -> usize {
    expected_framebuffer_pixels(width, height) * format.pixel_depth()
}

pub struct ApplicationWrapper<'app> {
    inner: Option<ApplicationWrapperInner>,
    app: &'app mut dyn ApplicationHandler,
}

impl<'app> ApplicationWrapper<'app> {
    pub fn new(app: &'app mut dyn ApplicationHandler) -> Self {
        Self { inner: None, app }
    }
}

impl<'app> winit::application::ApplicationHandler for ApplicationWrapper<'app> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.inner.is_none() {
            let inner = ApplicationWrapperInner::new(event_loop);
            self.inner = Some(inner);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(inner) = self.inner.as_mut() {
            inner.window_event(event_loop, window_id, event, self.app);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Some(inner) = self.inner.as_mut() {
            inner.device_event(event_loop, device_id, event, self.app);
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let Some(inner) = self.inner.as_mut() {
            inner.new_events(event_loop, cause);
        }
    }
}

struct ApplicationWrapperInner {
    window: Arc<winit::window::Window>,
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    window_title: String,
    surface: wgpu::Surface<'static>,
    surface_format_srgb: wgpu::TextureFormat,
    surface_format_linear: wgpu::TextureFormat,
    current_surface_format: wgpu::TextureFormat,
    texture_blitter: wgpu::util::TextureBlitter,
    staging_texture: Option<wgpu::Texture>,
    content_width: u32,
    content_height: u32,
    content_format: FramebufferFormat,
    is_occluded: bool,
    is_focused: bool,
    cursor_locked: bool,
    cursor_hidden: bool,
    exiting: bool,
}

impl ApplicationWrapperInner {
    fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let window_attributes = winit::window::Window::default_attributes()
            .with_title(DEFAULT_WINDOW_TITLE)
            .with_active(true);
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(event_loop.owned_display_handle()),
        ));
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .block_on()
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .block_on()
            .unwrap();

        let surface = instance.create_surface(window.clone()).unwrap();
        let capabilities = surface.get_capabilities(&adapter);
        if capabilities.formats.is_empty() {
            panic!("WebGPU adapter {adapter:?} is incompatible with surface {surface:?}");
        }
        let surface_format_srgb = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let surface_format_linear = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let current_surface_format = surface_format_srgb;

        let texture_blitter = Self::create_texture_blitter(&device, current_surface_format);

        let window_size = window.inner_size();

        let mut self_ = Self {
            window,
            instance,
            device,
            queue,
            window_title: String::from(DEFAULT_WINDOW_TITLE),
            surface,
            surface_format_srgb,
            surface_format_linear,
            current_surface_format,
            texture_blitter,
            staging_texture: None,
            content_width: window_size.width,
            content_height: window_size.height,
            content_format: FramebufferFormat::Rgba8UnormSrgb,
            is_occluded: false,
            is_focused: true,
            cursor_locked: false,
            cursor_hidden: false,
            exiting: false,
        };
        self_.configure_surface();
        self_
    }

    fn create_staging_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: FramebufferFormat,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.to_wgpu_format(),
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn create_texture_blitter(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::util::TextureBlitter {
        wgpu::util::TextureBlitterBuilder::new(device, format)
            .sample_type(wgpu::FilterMode::Nearest)
            .build()
    }

    fn configure_surface(&mut self) {
        let window_size = self.window.inner_size();
        let format = match self.content_format.is_srgb() {
            true => self.surface_format_srgb,
            false => self.surface_format_linear,
        };
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            view_formats: vec![format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: window_size.width,
            height: window_size.height,
            desired_maximum_frame_latency: 1, // no need for in-flight frames
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
        if self.current_surface_format != format {
            self.current_surface_format = format;
            self.texture_blitter = Self::create_texture_blitter(&self.device, format);
        }
    }

    /// Creates a new texture if:
    /// - `staging_texture` is `None`
    /// - existing staging texture's size is outdated compared to `content_{width|height}`
    fn get_or_create_staging_texture(
        &mut self,
        width: u32,
        height: u32,
        format: FramebufferFormat,
    ) -> wgpu::Texture {
        let needs_recreate = match &self.staging_texture {
            Some(staging_texture) => {
                let staging_texture_size = staging_texture.size();
                width != staging_texture_size.width
                    || height != staging_texture_size.height
                    || format.to_wgpu_format() != staging_texture.format()
            }
            None => true,
        };
        if needs_recreate {
            self.staging_texture = Some(Self::create_staging_texture(
                &self.device,
                width,
                height,
                format,
            ));
        }
        self.staging_texture.clone().unwrap()
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
        app: &mut dyn ApplicationHandler,
    ) {
        use winit::event::WindowEvent;

        if window_id != self.window.id() {
            return;
        }

        match event {
            WindowEvent::ActivationTokenDone { .. } => (),
            WindowEvent::Resized(_) => self.resized(app),
            WindowEvent::Moved(_) => (),
            WindowEvent::CloseRequested => {
                if app.exit_requested() {
                    event_loop.exit();
                }
            }
            WindowEvent::Destroyed => (),
            WindowEvent::DroppedFile(path) => {
                app.dropped_file(path);
            }
            WindowEvent::HoveredFile(path) => {
                app.hovered_file(path);
            }
            WindowEvent::HoveredFileCancelled => {
                app.hovered_file_canceled();
            }
            WindowEvent::Focused(focused) => {
                self.is_focused = focused;
                app.focus_changed(self.is_focused);
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                let key_code = match event.physical_key {
                    winit::keyboard::PhysicalKey::Code(key_code) => {
                        key_code::convert_winit_keycode(key_code)
                    }
                    winit::keyboard::PhysicalKey::Unidentified(_) => return,
                };
                match event.state {
                    winit::event::ElementState::Pressed => app.key_pressed(key_code, event.repeat),
                    winit::event::ElementState::Released => app.key_released(key_code),
                };
            }
            WindowEvent::ModifiersChanged(_) => (),
            WindowEvent::Ime(_) => (),
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let window_size = self.window.inner_size();
                let scale_x = self.content_width as f64 / window_size.width as f64;
                let scale_y = self.content_height as f64 / window_size.height as f64;
                let logical_x = position.x * scale_x;
                let logical_y = position.y * scale_y;
                app.mouse_moved_to_position(logical_x, logical_y);
            }
            WindowEvent::CursorEntered { .. } => {
                app.cursor_in_window(true);
            }
            WindowEvent::CursorLeft { .. } => {
                app.cursor_in_window(false);
            }
            WindowEvent::MouseWheel { .. } => (),
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                let button = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    _ => return,
                };
                match state {
                    winit::event::ElementState::Pressed => app.mouse_button_pressed(button),
                    winit::event::ElementState::Released => app.mouse_button_released(button),
                };
            }
            WindowEvent::PinchGesture { .. } => (),
            WindowEvent::PanGesture { .. } => (),
            WindowEvent::DoubleTapGesture { .. } => (),
            WindowEvent::RotationGesture { .. } => (),
            WindowEvent::TouchpadPressure { .. } => (),
            WindowEvent::AxisMotion { .. } => (),
            WindowEvent::Touch(_) => (),
            WindowEvent::ScaleFactorChanged { .. } => self.resized(app),
            WindowEvent::ThemeChanged(_) => (),
            WindowEvent::Occluded(occluded) => self.is_occluded = occluded,
            WindowEvent::RedrawRequested => {
                self.frame(app);
                self.window.request_redraw();
            }
        }
    }

    fn device_event(
        &mut self,
        _: &winit::event_loop::ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
        app: &mut dyn ApplicationHandler,
    ) {
        if !self.is_focused {
            return;
        }
        match event {
            winit::event::DeviceEvent::Added => (),
            winit::event::DeviceEvent::Removed => (),
            winit::event::DeviceEvent::MouseMotion { delta } => {
                app.mouse_moved_by_delta(delta.0, delta.1);
            }
            winit::event::DeviceEvent::MouseWheel { .. } => (),
            winit::event::DeviceEvent::Motion { .. } => (),
            winit::event::DeviceEvent::Button { .. } => (),
            winit::event::DeviceEvent::Key(_) => (),
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::event::StartCause,
    ) {
        if self.exiting {
            self.exiting = false;
            event_loop.exit();
        }
    }

    fn resized(&mut self, app: &mut dyn ApplicationHandler) {
        self.configure_surface();
        let size = self.window.inner_size();
        let dpi = self.window.scale_factor();
        app.window_resized(size.width, size.height, dpi);
    }

    fn acquire_next_frame(&mut self) -> Option<wgpu::SurfaceTexture> {
        use wgpu::CurrentSurfaceTexture;

        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => Some(frame),
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => None,
            CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                match self.surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(frame)
                    | CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
                    other => panic!("Failed to acquire next surface texture: {other:?}"),
                }
            }
            CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                match self.surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(frame)
                    | CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
                    other => panic!("Failed to acquire next surface texture: {other:?}"),
                }
            }
            CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.configure_surface();
                match self.surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(frame)
                    | CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
                    other => panic!("Failed to acquire next surface texture: {other:?}"),
                }
            }
        }
    }

    fn frame(&mut self, app: &mut dyn ApplicationHandler) {
        if self.is_occluded {
            return;
        }

        let window_size = self.window.inner_size();
        let dpi = self.window.scale_factor();
        let frame_output = app.redraw_requested(window_size.width, window_size.height, dpi);

        self.content_width = frame_output.framebuffer_width;
        self.content_height = frame_output.framebuffer_height;
        self.content_format = frame_output.framebuffer_format;

        self.handle_requests(frame_output);

        if !check_framebuffer_validity(frame_output) {
            std::hint::cold_path();
            self.present_framebuffer(
                1,
                1,
                FramebufferFormat::Rgba8UnormSrgb,
                &0xFF00FFFF_u32.to_be_bytes(),
            );
            return;
        }

        self.present_framebuffer(
            frame_output.framebuffer_width,
            frame_output.framebuffer_height,
            frame_output.framebuffer_format,
            frame_output.framebuffer_data,
        );
    }

    fn handle_requests(&mut self, frame_output: FrameOutput) {
        if let Some([width, height]) = frame_output.request_window_resize {
            let actual_size = self
                .window
                .request_inner_size(winit::dpi::LogicalSize::new(width, height));
            if let Some(actual_size) = actual_size {
                let actual_size_logical = actual_size.to_logical::<u32>(self.window.scale_factor());
                println!(
                    "requested window size {width}x{height} but actual granted size is only {}x{}",
                    actual_size_logical.width, actual_size_logical.height
                );
            }
        }

        self.exiting |= frame_output.request_exit;

        if frame_output.lock_cursor != self.cursor_locked {
            let grab_mode = match frame_output.lock_cursor {
                true => winit::window::CursorGrabMode::Locked,
                false => winit::window::CursorGrabMode::None,
            };
            if let Err(error) = self.window.set_cursor_grab(grab_mode) {
                eprintln!("cannot lock cursor: {error}");
            }
            self.cursor_locked = frame_output.lock_cursor;
        }

        if frame_output.hide_cursor != self.cursor_hidden {
            self.window.set_cursor_visible(!frame_output.hide_cursor);
            self.cursor_hidden = frame_output.hide_cursor;
        }

        if frame_output.window_title != self.window_title {
            self.window_title = String::from(frame_output.window_title);
            self.window.set_title(&self.window_title);
        }
    }

    fn present_framebuffer(
        &mut self,
        width: u32,
        height: u32,
        format: FramebufferFormat,
        data: &[u8],
    ) {
        if format.is_srgb() != self.current_surface_format.is_srgb() {
            self.configure_surface();
        }

        let Some(surface_texture) = self.acquire_next_frame() else {
            return;
        };

        assert_eq!(expected_framebuffer_size(width, height, format), data.len());
        let staging_texture = self.get_or_create_staging_texture(width, height, format);

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &staging_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(staging_texture.size().width * format.pixel_depth() as u32),
                rows_per_image: None,
            },
            staging_texture.size(),
        );
        let staging_texture_view = staging_texture.create_view(&Default::default());

        let surface_texture_view = surface_texture.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.texture_blitter.copy(
            &self.device,
            &mut encoder,
            &staging_texture_view,
            &surface_texture_view,
        );
        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        surface_texture.present();
    }
}

/// If invalid, `eprintln` with appropriate messages.
fn check_framebuffer_validity(frame_output: FrameOutput) -> bool {
    let mut framebuffer_valid = true;

    if frame_output.framebuffer_width == 0 || frame_output.framebuffer_height == 0 {
        eprintln!(
            "malformed framebuffer: width/height must be non-zero, but found {}x{}",
            frame_output.framebuffer_width, frame_output.framebuffer_height
        );
        framebuffer_valid = false;
    }

    let expected_framebuffer_size = expected_framebuffer_size(
        frame_output.framebuffer_width,
        frame_output.framebuffer_height,
        frame_output.framebuffer_format,
    );
    if expected_framebuffer_size != frame_output.framebuffer_data.len() {
        eprintln!(
            "malformed framebuffer: provided dimension {}x{}, but found framebuffer of {} bytes (expected {})",
            frame_output.framebuffer_width,
            frame_output.framebuffer_height,
            frame_output.framebuffer_data.len(),
            expected_framebuffer_size,
        );
        framebuffer_valid = false;
    }

    framebuffer_valid
}
