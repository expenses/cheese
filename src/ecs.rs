use ultraviolet::{Vec2, Vec3, Mat4};
use crate::renderer::{Instance, InstanceBuffers};
use crate::resources::{Camera, CameraControls, MouseState, ScreenDimensions};
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
#[read_component(Position)]
pub fn handle_mouse_click(
    #[resource] camera: &Camera,
    #[resource] mouse_state: &mut MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
    world: &SubWorld, commands: &mut CommandBuffer,
) {
    if !mouse_state.clicked {
        return;
    }

    let position = camera.cast_ray(mouse_state.position, screen_dimensions);

    <Entity>::query().for_each(world, |entity| {
        commands.remove_component::<Selected>(*entity)
    });

    let entity = <(Entity, &Position)>::query().iter(world)
        .filter(|(_, pos)| (position - pos.0).mag_sq() < 4.0)
        //.min_by_key(|(_, pos)| (position - pos.0).mag_sq());
        .next();

    if let Some((entity, _)) = entity {
        commands.add_component(*entity, Selected);
    }

    mouse_state.clicked = false;
}
