use super::*;
use crate::renderer::TorusInstance;

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
pub fn render_ui(#[resource] buffers: &mut InstanceBuffers, world: &SubWorld) {
    let text: String = <(Entity, &Health)>::query()
        .filter(component::<Selected>())
        .iter(world)
        .map(|(entity, health)| format!("{:?}: Health: {}\n", entity, health.0))
        .collect();

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
