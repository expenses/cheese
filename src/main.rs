mod renderer;
mod assets;
mod ecs;
mod resources;

use winit::{
	event::{
		ElementState, Event, KeyboardInput, WindowEvent, DeviceEvent,	
		VirtualKeyCode, MouseScrollDelta, MouseButton
	},
	event_loop::{ControlFlow, EventLoop},
	dpi::LogicalPosition,
};
use ultraviolet::{Vec2, Vec3};
use legion::*;
use crate::renderer::InstanceBuffers;
use crate::resources::{Camera, CameraControls, ScreenDimensions, MouseState, RtsControls};

fn main() -> anyhow::Result<()> {
	futures::executor::block_on(run())
}

async fn run() -> anyhow::Result<()> {
	env_logger::init();

	let event_loop = EventLoop::new();

	let mut imgui = imgui::Context::create();
	imgui.set_ini_filename(None);

	let (mut renderer, instance_buffers, screen_dimensions) = renderer::Renderer::new(&event_loop, &mut imgui).await?;

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
	resources.insert(RtsControls::default());

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
		.add_system(ecs::handle_left_click_system())
		.add_system(ecs::handle_right_click_system())
		.add_system(ecs::handle_rts_commands_system())
		.flush()
		.add_system(ecs::move_units_system())
		.add_system(ecs::render_boxes_system())
		.build();

	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::WindowEvent { ref event, .. } => {
				match event {
					WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
					WindowEvent::Resized(size) => {
						renderer.resize(size.width as u32, size.height as u32);
						let mut screen_dimensions = resources.get_mut::<ScreenDimensions>().unwrap();
						*screen_dimensions = ScreenDimensions { width: size.width as u32, height: size.height as u32 };
					},
					/*WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(code), .. }, ..} => {
						// Disabled due to a bug where a right keypress gets inserted at the start.
						
						let pressed = *state == ElementState::Pressed;

						let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();
						let mut rts_controls = resources.get_mut::<RtsControls>().unwrap();

						handle_key(code, pressed, &mut camera_controls, &mut rts_controls);
					},*/
					WindowEvent::MouseWheel { delta, .. } => {
						let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();

						camera_controls.zoom_delta += match delta {
							MouseScrollDelta::LineDelta(_, y) => y * 100.0,
							MouseScrollDelta::PixelDelta(LogicalPosition { y, .. }) => *y as f32
						};
					},
					WindowEvent::CursorMoved { position, .. } => {
						let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
						mouse_state.position = Vec2::new(position.x as f32, position.y as f32);
					},
					WindowEvent::MouseInput { state, button, .. } => {
						let pressed = *state == ElementState::Pressed;

						let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
						if pressed {
							match button {
								MouseButton::Left => mouse_state.left_clicked = true,
								MouseButton::Right => mouse_state.right_clicked = true,
								_ => {}
							}
						}
					}
					_ => {}
				}
			},
			Event::DeviceEvent { ref event, .. } => {
				match event {
					DeviceEvent::Key(KeyboardInput { state, virtual_keycode: Some(code), .. }) => {
						let pressed = *state == ElementState::Pressed;

						let mut camera_controls = resources.get_mut::<CameraControls>().unwrap();
						let mut rts_controls = resources.get_mut::<RtsControls>().unwrap();

						handle_key(code, pressed, &mut camera_controls, &mut rts_controls);
					},
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

				renderer.prepare_imgui(&mut imgui);
				let mut ui = imgui.frame();
				ecs::render_ui(&mut ui, &world);

				renderer.render(camera.to_matrix(), &mut instance_buffers, ui)
			},
			_ => {}
		}

		renderer.copy_event_to_imgui(&event, &mut imgui);
	});
}

fn handle_key(
	code: &VirtualKeyCode, pressed: bool,
	camera_controls: &mut CameraControls, rts_controls: &mut RtsControls,
) {
	log::debug!("{:?} pressed: {}", code, pressed);

	match code {
		VirtualKeyCode::Up    => camera_controls.up = pressed,
		VirtualKeyCode::Down  => camera_controls.down = pressed,
		VirtualKeyCode::Left  => camera_controls.left = pressed,
		VirtualKeyCode::Right => camera_controls.right = pressed,
		VirtualKeyCode::LShift => rts_controls.shift_held = pressed,
		VirtualKeyCode::S if pressed => rts_controls.s_pressed = true,
		_ => {}
	}
}
