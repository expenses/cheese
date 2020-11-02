mod renderer;
mod assets;
mod ecs;

use winit::{
	event::{ElementState, Event, KeyboardInput, WindowEvent, VirtualKeyCode, MouseScrollDelta},
	event_loop::{ControlFlow, EventLoop},
	dpi::PhysicalPosition,
};
use ultraviolet::{Vec2, Vec3, Mat4};
use legion::*;
use crate::renderer::InstanceBuffers;

fn main() -> anyhow::Result<()> {
	futures::executor::block_on(run())
}

async fn run() -> anyhow::Result<()> {
	env_logger::init();

	let event_loop = EventLoop::new();

	let (mut renderer, mut instance_buffers) = renderer::Renderer::new(&event_loop).await?;

	let mut world = World::default();
	let mut resources = Resources::default();
	resources.insert(instance_buffers);

	world.push((
		ecs::Position(Vec2::new(-1.0, 1.0)), ecs::Facing(1.0), ecs::Side::Green
	));

	world.push((
		ecs::Position(Vec2::new(1.0, -1.0)), ecs::Facing(-1.0), ecs::Side::Purple
	));

	world.push((
		ecs::Position(Vec2::new(5.0, -1.0)), ecs::Facing(-1.0), ecs::Side::Purple
	));

	let mut schedule = Schedule::builder()
		.add_system(ecs::render_boxes_system())
		.build();


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
					},
					WindowEvent::MouseWheel { delta, .. } => {
						camera_controls.zoom_delta += match delta {
							MouseScrollDelta::LineDelta(_, y) => y * 100.0,
							MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => y as f32
						};
					}
					_ => {}
				}
			},
			Event::MainEventsCleared => {
				let speed = 0.5;

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

				camera.position += (camera.looking_at - camera.position).normalized() * camera_controls.zoom_delta * 0.01;
				camera_controls.zoom_delta = 0.0;

				schedule.execute(&mut world, &mut resources);

				renderer.request_redraw()
			},
			Event::RedrawRequested(_) => {
				let mut instance_buffers = resources.get_mut::<InstanceBuffers>().unwrap();
				renderer.render(camera.to_matrix(), &mut instance_buffers)
			},
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
	zoom_delta: f32,
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
