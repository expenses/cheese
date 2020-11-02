use ultraviolet::{Vec2, Vec3, Mat4};
use crate::renderer::{Instance, InstanceBuffers};
use crate::resources::{Camera, CameraControls, MouseState, ScreenDimensions, RtsControls};
use legion::*;
use legion::world::SubWorld;
use legion::systems::CommandBuffer;

pub struct Position(pub Vec2);
pub struct Facing(pub f32);
pub enum Side {
    Green,
    Purple,
}
pub struct Selected;
pub struct MoveTo(Vec2);

#[legion::system(for_each)]
pub fn render_boxes(
    position: &Position, facing: &Facing, side: &Side, selected: Option<&Selected>,
    #[resource] buffers: &mut InstanceBuffers
) {
    let translation = Mat4::from_translation(Vec3::new(position.0.x, 0.0, position.0.y));
    let rotation = Mat4::from_rotation_y(facing.0);

    let instance = Instance {
        transform: translation * rotation,
        uv_flip: match side {
            Side::Green => 1.0,
            Side::Purple => -1.0,
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
        if !rts_controls.shift {
            <Entity>::query().filter(component::<Selected>()).for_each(world, |entity| {
                commands.remove_component::<Selected>(*entity)
            });
        }

        if rts_controls.shift && is_selected {
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
pub fn handle_right_click(
    #[resource] camera: &Camera,
    #[resource] mouse_state: &mut MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
    world: &SubWorld, commands: &mut CommandBuffer,
) {
    if !mouse_state.right_clicked {
        return;
    }

    let position = camera.cast_ray(mouse_state.position, screen_dimensions);

    <Entity>::query().filter(component::<Selected>())
        .for_each(world, |entity| {
            commands.add_component(*entity, MoveTo(position));
        });

    mouse_state.right_clicked = false;
}

#[legion::system(for_each)]
pub fn move_units(
    entity: &Entity,
    position: &mut Position,
    move_to: &MoveTo,
    commands: &mut CommandBuffer,
) {
    let speed = 0.1_f32;

    let direction = move_to.0 - position.0;

    if direction.mag_sq() <= speed.powi(2) {
        position.0 = move_to.0;
        commands.remove_component::<MoveTo>(*entity);
    } else {
        position.0 += direction.normalized() * speed;
    }
}
