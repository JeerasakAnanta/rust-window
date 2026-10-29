mod updater;

use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Default)]
struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title(format!("Rust Window v{}", updater::current_version()))
            .with_inner_size(LogicalSize::new(800.0, 600.0));
        let window = Rc::new(event_loop.create_window(attrs).expect("create window"));
        let context = Context::new(window.clone()).expect("create context");
        self.surface = Some(Surface::new(&context, window.clone()).expect("create surface"));
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
                    return;
                };
                let size = window.inner_size();
                let (Some(w), Some(h)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };
                surface.resize(w, h).expect("resize surface");
                let mut buffer = surface.buffer_mut().expect("get buffer");
                let width = size.width as usize;
                for (i, px) in buffer.iter_mut().enumerate() {
                    let x = (i % width) as u32 * 255 / size.width;
                    let y = (i / width) as u32 * 255 / size.height;
                    *px = (x << 16) | (y << 8) | 0x80;
                }
                buffer.present().expect("present buffer");
            }
            _ => {}
        }
    }
}

fn run_update_cli(apply: bool) -> Result<(), String> {
    let current = updater::current_version();
    println!("current version: {current}");
    match updater::check(&updater::api_url(), &current)? {
        None => println!("up to date"),
        Some(release) => {
            println!("new version available: {}", release.version);
            if apply {
                let dir = std::env::temp_dir().join("rust-window-update");
                let path = updater::download_verified(&release, &dir)?;
                println!("checksum OK, launching {}", path.display());
                updater::launch_installer(&path)?;
            }
        }
    }
    Ok(())
}

fn main() {
    let arg = std::env::args().nth(1);
    if let Some(flag) = arg
        .as_deref()
        .filter(|a| matches!(*a, "--check-update" | "--self-update"))
    {
        if let Err(e) = run_update_cli(flag == "--self-update") {
            eprintln!("update failed: {e}");
            std::process::exit(1);
        }
        return;
    }
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App::default()).expect("run app");
}
