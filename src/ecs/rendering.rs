use super::*;

#[legion::system(for_each)]
pub fn render_boxes(
    position: &Position,
    facing: &Facing,
    side: &Side,
    selected: Option<&Selected>,
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

    if selected.is_some() {
        buffers.selection_indicators.push(instance);
    }
}

pub fn render_ui(ui: &mut imgui::Ui, world: &World) {
    use imgui::im_str;

    let mut selected = <(Entity, &Position, &Health)>::query().filter(component::<Selected>());

    let window = imgui::Window::new(im_str!("Selected"));
    window
        .size([300.0, 100.0], imgui::Condition::FirstUseEver)
        .build(&ui, || {
            selected.iter(world).for_each(|(entity, position, health)| {
                ui.text(im_str!(
                    "{:?}: {:?}, Health: {}",
                    entity,
                    position.0,
                    health.0
                ))
            });
        });
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
