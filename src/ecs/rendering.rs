use super::*;
use crate::animation::Skin;
use crate::renderer::{
    Font, LineBuffers, ModelBuffers, ModelInstance, TextBuffer, TorusBuffer, TorusInstance,
};
use crate::resources::{CommandMode, CursorIcon, DpiScaling, RayCastLocation};
use ultraviolet::Vec4;

const COLOUR_MAX: Vec3 = Vec3::new(255.0, 255.0, 255.0);
const GREEN: Vec3 = Vec3::new(43.0, 140.0, 0.0);
const PURPLE: Vec3 = Vec3::new(196.0, 0.0, 109.0);

fn mix(colour_a: Vec3, colour_b: Vec3, factor: f32) -> Vec3 {
    colour_a * (1.0 - factor) + colour_b * factor
}

#[legion::system]
pub fn render_building_plan(
    #[resource] ray_cast_location: &RayCastLocation,
    #[resource] rts_controls: &RtsControls,
    #[resource] model_buffers: &mut ModelBuffers,
) {
    if rts_controls.mode != CommandMode::Construct {
        model_buffers.building_plan.clear();
        return;
    }

    model_buffers.building_plan.set(
        Building::Pump,
        ModelInstance {
            transform: Mat4::from_translation(Vec3::new(
                ray_cast_location.0.x,
                0.0,
                ray_cast_location.0.y,
            )),
            flat_colour: Vec4::new(0.0, 1.0, 0.0, 0.25),
        },
    );
}

#[legion::system(for_each)]
pub fn render_units(
    position: &Position,
    side: &Side,
    facing: &Facing,
    skin: &Skin,
    unit: &Unit,
    #[resource] model_buffers: &mut ModelBuffers,
) {
    let translation = Mat4::from_translation(Vec3::new(position.0.x, 0.0, position.0.y));
    let rotation = Mat4::from_rotation_y(facing.0);

    let (instance_buffer, joint_buffer) = match unit {
        Unit::MouseMarine => (
            &mut model_buffers.mice_marines,
            &mut model_buffers.mice_marines_joints,
        ),
        Unit::Engineer => (
            &mut model_buffers.mice_engineers,
            &mut model_buffers.mice_engineers_joints,
        ),
    };

    instance_buffer.push(ModelInstance {
        transform: translation * rotation,
        flat_colour: {
            let colour = match side {
                Side::Green => GREEN,
                Side::Purple => PURPLE,
            } / COLOUR_MAX;
            let colour = mix(colour, Vec3::new(1.0, 1.0, 1.0), 0.25);

            Vec4::new(colour.x, colour.y, colour.z, 0.2)
        },
    });
    for joint in &skin.joints {
        joint_buffer.push(joint.matrix);
    }
}

#[legion::system(for_each)]
#[filter(component::<Selected>())]
pub fn render_selections(
    position: &Position,
    side: &Side,
    radius: &Radius,
    #[resource] torus_buffer: &mut TorusBuffer,
) {
    torus_buffer.toruses.push(TorusInstance {
        center: Vec3::new(position.0.x, 0.0, position.0.y),
        colour: match side {
            Side::Green => GREEN / COLOUR_MAX,
            Side::Purple => PURPLE / COLOUR_MAX,
        },
        radius: radius.0,
    });
}

#[legion::system]
#[read_component(Position)]
#[read_component(Radius)]
#[read_component(Side)]
pub fn render_under_select_box(
    #[resource] mouse_state: &MouseState,
    #[resource] camera: &Camera,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] player_side: &PlayerSide,
    #[resource] torus_buffer: &mut TorusBuffer,
    world: &SubWorld,
) {
    if let Some(start) = mouse_state.left_state.is_being_dragged() {
        let select_box = SelectBox::new(camera, screen_dimensions, start, mouse_state.position);

        <(&Position, &Radius, &Side)>::query()
            .filter(component::<Selectable>() & !component::<Selected>())
            .iter(world)
            .filter(|(.., side)| **side == player_side.0)
            .filter(|(position, ..)| select_box.contains(position.0))
            .for_each(|(position, radius, _)| {
                torus_buffer.toruses.push(TorusInstance {
                    center: Vec3::new(position.0.x, 0.0, position.0.y),
                    colour: Vec3::new(1.0, 1.0, 1.0),
                    radius: radius.0,
                });
            });
    }
}

#[legion::system(for_each)]
pub fn render_health_bars(
    position: &Position,
    health: &Health,
    unit: Option<&Unit>,
    building: Option<&Building>,
    #[resource] camera: &Camera,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
    #[resource] line_buffers: &mut LineBuffers,
) {
    let stats = {
        let unit_stats = unit.map(|unit| {
            let stats = unit.stats();
            (stats.max_health, stats.health_bar_height)
        });
        let building_stats = building.map(|building| {
            let stats = building.stats();
            (stats.max_health, 5.0)
        });
        unit_stats.or(building_stats)
    };

    if let Some((max_health, health_bar_height)) = stats {
        if health.0 != max_health {
            let floating = Vec3::new(position.0.x, health_bar_height, position.0.y);
            let location = screen_location(floating, camera, screen_dimensions);

            let health_percentage = health.0 as f32 / max_health as f32;
            let length = 60.0 * health_percentage;

            line_buffers.draw_filled_rect(
                location,
                Vec2::new(length + 2.0, 12.0),
                Vec3::new(0.0, 0.0, 0.0),
                dpi_scaling.0,
            );

            line_buffers.draw_filled_rect(
                location,
                Vec2::new(length, 10.0),
                Vec3::new(1.0 - health_percentage, health_percentage, 0.0),
                dpi_scaling.0,
            );
        }
    }
}

fn screen_location(position: Vec3, camera: &Camera, screen_dimensions: &ScreenDimensions) -> Vec2 {
    let &ScreenDimensions { width, height } = screen_dimensions;
    let view = camera.to_matrix();
    let perspective = crate::renderer::create_perspective_mat4(width, height);
    let screen_position = perspective * view * Vec4::new(position.x, position.y, position.z, 1.0);
    let screen_position = Vec2::new(screen_position.x, screen_position.y) / screen_position.w;
    wgpu_to_screen(screen_position, width as f32, height as f32)
}

fn wgpu_to_screen(wgpu: Vec2, width: f32, height: f32) -> Vec2 {
    Vec2::new((wgpu.x + 1.0) / 2.0 * width, (1.0 - wgpu.y) / 2.0 * height)
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Health)]
pub fn render_ui(
    #[resource] rts_controls: &RtsControls,
    #[resource] dpi_scaling: &DpiScaling,
    #[resource] text_buffer: &mut TextBuffer,
    world: &SubWorld,
) {
    let mode = Some(format!("Mode: {:?}\n", rts_controls.mode)).into_iter();

    let mut query = <(Entity, &Health)>::query().filter(component::<Selected>());

    let unit_info = query
        .iter(world)
        .map(|(entity, health)| format!("{:?}: Health: {}\n", entity, health.0));

    let text: String = mode.chain(unit_info).collect();

    text_buffer.render_text(
        Vec2::new(10.0, 10.0),
        &text,
        Font::Ui,
        1.0,
        dpi_scaling.0,
        false,
        Vec4::one(),
    );
}

#[legion::system(for_each)]
#[filter(component::<Selected>() & component::<Position>())]
#[read_component(Position)]
pub fn render_command_paths(
    queue: &CommandQueue,
    entity: &Entity,
    side: &Side,
    #[resource] model_buffers: &mut ModelBuffers,
    #[resource] player_side: &PlayerSide,
    world: &SubWorld,
) {
    let position = <&Position>::query()
        .get(world, *entity)
        .expect("We've applied a filter to this system for Position");

    if *side != player_side.0 {
        // Can't be leaking infomation about what enemy units are doing!
        return;
    }

    let mut prev = position.0;

    for command in queue.0.iter() {
        let position = match command {
            Command::MoveTo { target, .. } => Some(*target),
            Command::Attack {
                target,
                explicit: true,
                ..
            } => {
                let position = <&Position>::query()
                    .get(world, *target)
                    .expect("We've cancelled actions on dead entities")
                    .0;
                Some(position)
            }
            Command::Build { target, .. } => {
                let position = <&Position>::query()
                    .get(world, *target)
                    .expect("We've cancelled actions on dead entities")
                    .0;
                Some(position)
            }
            Command::Attack {
                explicit: false, ..
            } => None,
        };

        let move_colour = Vec4::new(0.25, 0.25, 1.0, 1.0);
        let attack_colour = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let build_colour = Vec4::new(0.25, 1.0, 0.25, 1.0);

        let colour = match command {
            Command::MoveTo { attack_move, .. } => {
                if *attack_move {
                    attack_colour
                } else {
                    move_colour
                }
            }
            Command::Attack { .. } => attack_colour,
            Command::Build { .. } => build_colour,
        };

        if let Some(position) = position {
            model_buffers.command_indicators.push(ModelInstance {
                transform: Mat4::from_translation(Vec3::new(position.x, 0.01, position.y)),
                flat_colour: colour,
            });

            let center = (prev + position) / 2.0;
            let vector = position - prev;
            let rotation = vector.y.atan2(vector.x);
            let scale = vector.mag();

            model_buffers.command_paths.push(ModelInstance {
                transform: Mat4::from_translation(Vec3::new(center.x, 0.005, center.y))
                    * Mat4::from_rotation_y(rotation)
                    * Mat4::from_nonuniform_scale(Vec3::new(scale, 1.0, 1.0)),
                flat_colour: colour,
            });

            prev = position;
        }
    }
}

#[legion::system(for_each)]
pub fn render_buildings(
    position: &Position,
    building: &Building,
    building_completeness: &BuildingCompleteness,
    skin: Option<&Skin>,
    #[resource] model_buffers: &mut ModelBuffers,
) {
    let buffer = match building {
        Building::Armoury => &mut model_buffers.armouries,
        Building::Pump => &mut model_buffers.pumps,
    };

    let scale = (building_completeness.0 as f32 / building.stats().max_health as f32).max(0.01);

    buffer.push(ModelInstance {
        transform: Mat4::from_translation(Vec3::new(position.0.x, 0.0, position.0.y))
            * Mat4::from_nonuniform_scale(Vec3::new(1.0, scale, 1.0)),
        flat_colour: Vec4::new(1.0, 1.0, 1.0, 1.0),
    });

    if let Some(skin) = skin {
        for joint in &skin.joints {
            model_buffers.pump_joints.push(joint.matrix);
        }
    }
}

#[legion::system]
pub fn render_drag_box(
    #[resource] mouse_state: &MouseState,
    #[resource] dpi_scaling: &DpiScaling,
    #[resource] line_buffers: &mut LineBuffers,
) {
    if let Some(start) = mouse_state.left_state.is_being_dragged() {
        let (top_left, bottom_right) = sort_points(start, mouse_state.position);
        line_buffers.draw_rect(top_left, bottom_right, dpi_scaling.0);
    }
}

#[legion::system(for_each)]
#[filter(component::<Bullet>())]
pub fn render_bullets(
    position: &Position,
    facing: &Facing,
    #[resource] model_buffers: &mut ModelBuffers,
) {
    let gun_height = 1.8;
    let translation = Mat4::from_translation(Vec3::new(position.0.x, gun_height, position.0.y));
    let rotation = Mat4::from_rotation_y(facing.0);

    model_buffers.bullets.push(ModelInstance {
        transform: translation * rotation,
        flat_colour: Vec4::one(),
    });
}

#[legion::system]
#[read_component(Position)]
#[read_component(Radius)]
pub fn render_unit_under_cursor(
    #[resource] ray_cast_location: &RayCastLocation,
    #[resource] cursor_icon: &mut CursorIcon,
    #[resource] torus_buffer: &mut TorusBuffer,
    world: &SubWorld,
) {
    if let Some((pos, radius)) = unit_under_cursor(ray_cast_location, world) {
        cursor_icon.0 = winit::window::CursorIcon::Hand;
        torus_buffer.toruses.push(TorusInstance {
            center: Vec3::new(pos.x, 0.0, pos.y),
            colour: Vec3::new(1.0, 1.0, 1.0),
            radius,
        });
    }
}

fn unit_under_cursor(ray_cast_location: &RayCastLocation, world: &SubWorld) -> Option<(Vec2, f32)> {
    let position = ray_cast_location.0;

    <(&Position, &Radius)>::query()
        .iter(world)
        .find(|(pos, radius)| (position - pos.0).mag_sq() < radius.0.powi(2))
        .map(|(pos, radius)| (pos.0, radius.0))
}
