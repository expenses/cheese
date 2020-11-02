mod renderer;
mod assets;

use winit::{
	event::{ElementState, Event, KeyboardInput, WindowEvent, VirtualKeyCode},
	event_loop::{ControlFlow, EventLoop},
};
use ultraviolet::{Vec3, Mat4};

fn main() -> anyhow::Result<()> {
	futures::executor::block_on(run())
}

async fn run() -> anyhow::Result<()> {
	let event_loop = EventLoop::new();

	let (mut renderer, mut instance_buffers) = renderer::Renderer::new(&event_loop).await?;

	let mut camera = Camera {
		position: Vec3::new(0.0, 20.0, 10.0),
		looking_at: Vec3::new(0.0, 0.0, 0.0),
	};

	let mut camera_controls = CameraControls::default();

	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::WindowEvent { event, .. } => {
				match event {
					WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
					WindowEvent::Resized(size) => {
						renderer.resize(size.width as u32, size.height as u32);
					},
					WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(code), .. }, ..} => {
						let pressed = state == ElementState::Pressed;

						match code {
							VirtualKeyCode::Up    => camera_controls.up = pressed,
							VirtualKeyCode::Down  => camera_controls.down = pressed,
							VirtualKeyCode::Left  => camera_controls.left = pressed,
							VirtualKeyCode::Right => camera_controls.right = pressed,
							_ => {}
						}
					}
					_ => {}
				}
			},
			Event::MainEventsCleared => {
				let speed = 0.1;

				let right = Vec3::new(speed, 0.0, 0.0);
				let forwards = Vec3::new(0.0, 0.0, -speed);

				if camera_controls.left {
					camera.position -= right;
					camera.looking_at -= right;
				}

				if camera_controls.right {
					camera.position += right;
					camera.looking_at += right;
				}

				if camera_controls.up {
					camera.position += forwards;
					camera.looking_at += forwards;
				}

				if camera_controls.down {
					camera.position -= forwards;
					camera.looking_at -= forwards;
				}

				renderer.request_redraw()
			},
			Event::RedrawRequested(_) => renderer.render(camera.to_matrix(), &mut instance_buffers),
			_ => {}
		}
	});
}

#[derive(Default)]
struct CameraControls {
	up: bool,
	down: bool,
	left: bool,
	right: bool,
}

struct Camera {
	position: Vec3,
	looking_at: Vec3,
}

impl Camera {
	fn to_matrix(&self) -> Mat4 {
		Mat4::look_at(
			self.position,
			self.looking_at,
			Vec3::new(0.0, 1.0, 0.0)
		)
	}
}
