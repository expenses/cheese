mod renderer;
mod assets;
mod ecs;
mod resources;

use winit::{
	event::{ElementState, Event, KeyboardInput, WindowEvent, VirtualKeyCode, MouseScrollDelta, MouseButton},
	event_loop::{ControlFlow, EventLoop},
	dpi::PhysicalPosition,
};
use ultraviolet::{Vec2, Vec3};
use legion::*;
use crate::renderer::InstanceBuffers;
use crate::resources::{Camera, CameraControls, ScreenDimensions, MouseState};

fn main() -> anyhow::Result<()> {
	futures::executor::block_on(run())
}

async fn run() -> anyhow::Result<()> {
	env_logger::init();

	let event_loop = EventLoop::new();

	let (mut renderer, instance_buffers, screen_dimensions) = renderer::Renderer::new(&event_loop).await?;

	let mut world = World::default();
	let mut resources = Resources::default();
	resources.insert(instance_buffers);
	resources.insert(screen_dimensions);
	resources.insert(CameraControls::default());
	resources.insert(Camera {
		position: Vec3::new(0.0, 20.0, 10.0),
		looking_at: Vec3::new(0.0, 0.0, 0.0),
	});
	resources.insert(MouseState::default());

	world.push((
		ecs::Position(Vec2::new(0.0, 1.0)), ecs::Facing(1.0), ecs::Side::Green,
	));

	world.push((
		ecs::Position(Vec2::new(1.0, -1.0)), ecs::Facing(-1.0), ecs::Side::Purple
	));

	world.push((
		ecs::Position(Vec2::new(5.0, -1.0)), ecs::Facing(-1.0), ecs::Side::Purple
	));

	let mut schedule = Schedule::builder()
		.add_system(ecs::control_camera_system())
		.add_system(ecs::handle_mouse_click_system())
		.add_system(ecs::render_boxes_system())
		.build();


	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::WindowEvent { event, .. } => {
				match event {
					WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
					WindowEvent::Resized(size) => {
						renderer.resize(size.width as u32, size.height as u32);
						let mut screen_dimensions = resources.get_mut::<ScreenDimensions>().unwrap();
						*screen_dimensions = ScreenDimensions { width: size.width as u32, height: size.height as u32 };
					},
					WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(code), .. }, ..} => {
						let pressed = state == ElementState::Pressed;

						let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();

						match code {
							VirtualKeyCode::Up    => camera_controls.up = pressed,
							VirtualKeyCode::Down  => camera_controls.down = pressed,
							VirtualKeyCode::Left  => camera_controls.left = pressed,
							VirtualKeyCode::Right => camera_controls.right = pressed,
							_ => {}
						}
					},
					WindowEvent::MouseWheel { delta, .. } => {
						let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();

						camera_controls.zoom_delta += match delta {
							MouseScrollDelta::LineDelta(_, y) => y * 100.0,
							MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => y as f32
						};
					},
					WindowEvent::CursorMoved { position, .. } => {
						let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
						mouse_state.position = Vec2::new(position.x as f32, position.y as f32);
					},
					WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
						let pressed = state == ElementState::Pressed;

						let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
						if pressed {
							mouse_state.clicked = true;
						}
					}
					_ => {}
				}
			},
			Event::MainEventsCleared => {
				schedule.execute(&mut world, &mut resources);

				renderer.request_redraw()
			},
			Event::RedrawRequested(_) => {
				let mut instance_buffers = resources.get_mut::<InstanceBuffers>().unwrap();
				let camera = resources.get::<Camera>().unwrap();
				renderer.render(camera.to_matrix(), &mut instance_buffers)
			},
			_ => {}
		}
	});
}
