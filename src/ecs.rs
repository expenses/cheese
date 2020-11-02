use ultraviolet::{Vec2, Vec3, Mat4};
use crate::renderer::{Instance, InstanceBuffers, Vertex};
use crate::resources::{Camera, CameraControls, MouseState, ScreenDimensions, RtsControls};
use legion::*;
use legion::world::SubWorld;
use legion::systems::CommandBuffer;
use std::collections::VecDeque;

pub struct Position(pub Vec2);
pub struct Facing(pub f32);
pub enum Side {
	Green,
	Purple,
}
pub struct Selected;

#[derive(Clone)]
pub enum Command {
	MoveTo(Vec2),
}

#[derive(Default)]
pub struct CommandQueue(VecDeque<Command>);

pub struct Avoidance(pub Vec2);

#[legion::system(for_each)]
pub fn render_boxes(
	position: &Position, facing: &Facing, side: &Side, selected: Option<&Selected>,
	#[resource] buffers: &mut InstanceBuffers
) {
	let translation = Mat4::from_translation(Vec3::new(position.0.x, 0.0, position.0.y));
	let rotation = Mat4::from_rotation_y(facing.0);

	let instance = Instance {
		transform: translation * rotation,
		uv_x_offset: match side {
			Side::Green => 0.0,
			Side::Purple => 0.5,
		}
	};

	buffers.mice.push(instance);

	if selected.is_some() {
		buffers.selection_indicators.push(instance);
	}
}

#[legion::system]
pub fn control_camera(
	#[resource] camera: &mut Camera, #[resource] camera_controls: &mut CameraControls,
) {
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
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Selected)]
#[read_component(Position)]
pub fn handle_left_click(
	#[resource] camera: &Camera,
	#[resource] mouse_state: &mut MouseState,
	#[resource] screen_dimensions: &ScreenDimensions,
	#[resource] rts_controls: &RtsControls,
	world: &SubWorld, commands: &mut CommandBuffer,
) {
	if !mouse_state.left_clicked {
		return;
	}

	let position = camera.cast_ray(mouse_state.position, screen_dimensions);

	let entity = <(Entity, &Position, Option<&Selected>)>::query().iter(world)
		.filter(|(_, pos, _)| (position - pos.0).mag_sq() < 4.0)
		//.min_by_key(|(_, pos)| (position - pos.0).mag_sq());
		.next()
		.map(|(entity, _, selected)| (entity, selected.is_some()));

	if let Some((entity, is_selected)) = entity {
		if !rts_controls.shift_held {
			<Entity>::query().filter(component::<Selected>()).for_each(world, |entity| {
				commands.remove_component::<Selected>(*entity)
			});
		}

		if rts_controls.shift_held && is_selected {
			commands.remove_component::<Selected>(*entity);
		} else {
			commands.add_component(*entity, Selected);
		}
	}

	mouse_state.left_clicked = false;
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Selected)]
#[write_component(CommandQueue)]
pub fn handle_right_click(
	#[resource] camera: &Camera,
	#[resource] mouse_state: &mut MouseState,
	#[resource] screen_dimensions: &ScreenDimensions,
	#[resource] rts_controls: &RtsControls,
	world: &mut SubWorld,
) {
	if !mouse_state.right_clicked {
		return;
	}

	let position = camera.cast_ray(mouse_state.position, screen_dimensions);

	<&mut CommandQueue>::query().filter(component::<Selected>())
		.for_each_mut(world, |commands| {
			if !rts_controls.shift_held {
				commands.0.clear();
			}

			commands.0.push_back(Command::MoveTo(position));
		});

	mouse_state.right_clicked = false;
}

#[legion::system(for_each)]
pub fn move_units(
	position: &mut Position,
	commands: &mut CommandQueue,
) {
	let speed = 0.1_f32;

	match commands.0.front().clone() {
		Some(Command::MoveTo(target)) => {
			let direction = *target - position.0;

			if direction.mag_sq() <= speed.powi(2) {
				position.0 = *target;
				commands.0.pop_front();
			} else {
				position.0 += direction.normalized() * speed;
			}
		},
		None => {}
	}
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Selected)]
#[write_component(CommandQueue)]
pub fn handle_rts_commands(
	#[resource] rts_controls: &mut RtsControls,
	world: &mut SubWorld,
) {
	if !rts_controls.s_pressed {
		return;
	}

	<&mut CommandQueue>::query().filter(component::<Selected>())
		.for_each_mut(world, |commands| commands.0.clear());

	rts_controls.s_pressed = false;
}

pub fn render_ui(ui: &mut imgui::Ui, world: &World) {
	use imgui::im_str;

	let mut selected = <(Entity, &Position)>::query().filter(component::<Selected>());

	let window = imgui::Window::new(im_str!("Selected"));
	window
		.size([300.0, 100.0], imgui::Condition::FirstUseEver)
		.build(&ui, || {
			selected.iter(world).for_each(|(entity, position)| {
				ui.text(im_str!("{:?}: {:?}", entity, position.0))
			});
		});
}

#[legion::system(for_each)]
#[filter(component::<Selected>())]
pub fn render_command_paths(
	queue: &CommandQueue,
	position: &Position,
	side: &Side,
	#[resource] buffers: &mut InstanceBuffers,
) {
	let uv = match side {
		Side::Green => Vec2::new(0.5 / 64.0, 0.5),
		Side::Purple => Vec2::new(1.5 / 64.0, 0.5)
	};

	let mut prev = position_to_vertex(position.0, uv);

	for command in queue.0.iter() {
		let position = match command {
			Command::MoveTo(position) => *position
		};

		let vertex = position_to_vertex(position, uv);

		buffers.command_paths.push(prev);
		buffers.command_paths.push(vertex);

		prev = vertex;
	}
}

fn position_to_vertex(pos: Vec2, uv: Vec2) -> Vertex {
	Vertex {
		position: Vec3::new(pos.x, 0.1, pos.y),
		normal: Vec3::new(0.0, 0.0, 0.0),
		uv
	}
}

#[legion::system]
#[read_component(Position)]
pub fn avoidance(
	world: &SubWorld,
	command_buffer: &mut CommandBuffer,
) {
	let desired_seperation = 2.0_f32;

	<(Entity, &Position)>::query().for_each(world, |(entity, position)| {

		let mut avoidance_direction = Vec2::new(0.0, 0.0);
		let mut count = 0;

		for other_position in <&Position>::query().iter(world) {
			let away_vector = position.0 - other_position.0;
			let distance_sq = away_vector.mag_sq();
			
			if distance_sq > 0.0 && distance_sq < desired_seperation.powi(2) {
				let distance = distance_sq.sqrt();

				avoidance_direction += away_vector.normalized() / distance;
				count += 1;
			}
		}

		if count > 0 {
			avoidance_direction /= count as f32;
			command_buffer.add_component(*entity, Avoidance(avoidance_direction));
		}
	})
}

#[legion::system(for_each)]
pub fn apply_steering(
	entity: &Entity,
	position: &mut Position,
	avoidance: &Avoidance,
	command_buffer: &mut CommandBuffer,
) {
	position.0 += avoidance.0 * 0.1;
	command_buffer.remove_component::<Avoidance>(*entity);
}

#[legion::system]
pub fn draw_lines(
	#[resource] buffers: &mut InstanceBuffers,
) {
	buffers.line_buffers.draw_rect(Vec2::new(100.0, 100.0), Vec2::new(200.0, 200.0));
}
