use super::*;
use crate::renderer::TorusInstance;
use crate::resources::CursorIcon;
use ultraviolet::Vec4;

const COLOUR_MAX: Vec3 = Vec3::new(255.0, 255.0, 255.0);
const GREEN: Vec3 = Vec3::new(43.0, 140.0, 0.0);
const PURPLE: Vec3 = Vec3::new(196.0, 0.0, 109.0);

#[legion::system(for_each)]
pub fn render_boxes(
    position: &Position,
    facing: &Facing,
    side: &Side,
    #[resource] buffers: &mut InstanceBuffers,
) {
    let translation = Mat4::from_translation(Vec3::new(position.0.x, 0.0, position.0.y));
    let rotation = Mat4::from_rotation_y(facing.0);

    let instance = Instance {
        transform: translation * rotation,
        uv_x_offset: match side {
            Side::Green => 0.0,
            Side::Purple => 0.5,
        },
    };

    buffers.mice.push(instance);
}

#[legion::system(for_each)]
#[filter(component::<Selected>())]
pub fn render_selections(
    position: &Position,
    side: &Side,
    radius: &Radius,
    #[resource] buffers: &mut InstanceBuffers,
) {
    buffers.toruses.push(TorusInstance {
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
    #[resource] buffers: &mut InstanceBuffers,
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
                buffers.toruses.push(TorusInstance {
                    center: Vec3::new(position.0.x, 0.0, position.0.y),
                    colour: Vec3::new(1.0, 1.0, 1.0),
                    radius: radius.0,
                })
            });
    }
}

#[legion::system(for_each)]
pub fn render_health_bars(
    position: &Position,
    radius: &Radius,
    health: &Health,
    unit: &Unit,
    #[resource] camera: &Camera,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] buffers: &mut InstanceBuffers,
) {
    let stats = unit.stats();

    if health.0 != stats.max_health {
        let floating = Vec3::new(position.0.x, radius.0 * 2.0, position.0.y);
        let location = screen_location(floating, camera, screen_dimensions);

        let health_percentage = health.0 as f32 / stats.max_health as f32;
        let length = 60.0 * health_percentage;

        buffers.line_buffers.draw_filled_rect(
            location,
            Vec2::new(length, 10.0),
            Vec3::new(1.0 - health_percentage, health_percentage, 0.0),
        );
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

#[legion::system(for_each)]
#[filter(component::<Selected>())]
pub fn render_firing_ranges(
    position: &Position,
    firing_range: &FiringRange,
    side: &Side,
    #[resource] player_side: &PlayerSide,
    #[resource] buffers: &mut InstanceBuffers,
) {
    if *side != player_side.0 {
        return;
    }

    buffers.toruses.push(TorusInstance {
        center: Vec3::new(position.0.x, 0.0, position.0.y),
        colour: Vec3::new(0.5, 0.0, 0.0),
        radius: firing_range.0,
    });
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Health)]
pub fn render_ui(
    #[resource] rts_controls: &RtsControls,
    #[resource] buffers: &mut InstanceBuffers,
    world: &SubWorld,
) {
    let mode = Some(format!("Mode: {:?}\n", rts_controls.mode)).into_iter();

    let mut query = <(Entity, &Health)>::query().filter(component::<Selected>());

    let unit_info = query
        .iter(world)
        .map(|(entity, health)| format!("{:?}: Health: {}\n", entity, health.0));

    let text: String = mode.chain(unit_info).collect();

    buffers.render_text((10.0, 10.0), &text);
}

#[legion::system(for_each)]
#[filter(component::<Selected>() & component::<Position>())]
#[read_component(Position)]
pub fn render_command_paths(
    queue: &CommandQueue,
    entity: &Entity,
    side: &Side,
    #[resource] buffers: &mut InstanceBuffers,
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

    let uv = match side {
        Side::Green => Vec2::new(0.5 / 64.0, 0.5),
        Side::Purple => Vec2::new(1.5 / 64.0, 0.5),
    };

    let mut prev = position_to_vertex(position.0, uv);

    for command in queue.0.iter() {
        let position = match command {
            Command::MoveTo(position) => *position,
            Command::AttackMove(position) => *position,
            Command::Attack(target) => {
                <&Position>::query()
                    .get(world, *target)
                    .expect("We've cancelled attack commands on dead entities")
                    .0
            }
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
        uv,
    }
}

#[legion::system]
pub fn render_drag_box(
    #[resource] mouse_state: &MouseState,
    #[resource] buffers: &mut InstanceBuffers,
) {
    if let Some(start) = mouse_state.left_state.is_being_dragged() {
        let (top_left, bottom_right) = sort_points(start, mouse_state.position);
        buffers.line_buffers.draw_rect(top_left, bottom_right);
    }
}

#[legion::system(for_each)]
#[filter(component::<Bullet>())]
pub fn render_bullets(
    position: &Position,
    facing: &Facing,
    #[resource] buffers: &mut InstanceBuffers,
) {
    let translation = Mat4::from_translation(Vec3::new(position.0.x, 1.0, position.0.y));
    let rotation = Mat4::from_rotation_y(facing.0);

    buffers.bullets.push(Instance {
        transform: translation * rotation,
        uv_x_offset: 0.0,
    });
}

#[legion::system]
#[read_component(Position)]
#[read_component(Radius)]
pub fn set_cursor_if_unit_under(
    #[resource] camera: &Camera,
    #[resource] mouse_state: &MouseState,
    #[resource] screen_position: &ScreenDimensions,
    #[resource] cursor_icon: &mut CursorIcon,
    world: &SubWorld,
) {
    if unit_under_cursor(camera, mouse_state, screen_position, world) {
        cursor_icon.0 = winit::window::CursorIcon::Hand;
    }
}

fn unit_under_cursor(
    camera: &Camera,
    mouse_state: &MouseState,
    screen_dimensions: &ScreenDimensions,
    world: &SubWorld,
) -> bool {
    let position = camera.cast_ray(mouse_state.position, screen_dimensions);

    <(&Position, &Radius)>::query()
        .iter(world)
        .any(|(pos, radius)| (position - pos.0).mag_sq() < radius.0.powi(2))
}
