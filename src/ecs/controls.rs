use super::*;

#[legion::system]
pub fn control_camera(
    #[resource] camera: &mut Camera,
    #[resource] camera_controls: &mut CameraControls,
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

    camera.position +=
        (camera.looking_at - camera.position).normalized() * camera_controls.zoom_delta * 0.01;
    camera_controls.zoom_delta = 0.0;
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Selected)]
#[read_component(Position)]
#[read_component(Side)]
pub fn handle_left_click(
    #[resource] camera: &Camera,
    #[resource] mouse_state: &MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] rts_controls: &RtsControls,
    #[resource] player_side: &PlayerSide,
    world: &SubWorld,
    commands: &mut CommandBuffer,
) {
    if !mouse_state.left_state.was_clicked() {
        return;
    }

    let position = camera.cast_ray(mouse_state.position, screen_dimensions);

    let entity = <(Entity, &Position, Option<&Selected>, &Side)>::query()
        .filter(component::<Selectable>())
        .iter(world)
        .filter(|(_, pos, ..)| (position - pos.0).mag_sq() < 4.0)
        //.min_by_key(|(_, pos)| (position - pos.0).mag_sq());
        .next()
        .map(|(entity, _, selected, side)| (entity, selected.is_some(), side));

    if let Some((entity, is_selected, side)) = entity {
        if !rts_controls.shift_held {
            deselect_all(world, commands);
        }

        if rts_controls.shift_held && is_selected {
            commands.remove_component::<Selected>(*entity);
        } else if !rts_controls.shift_held {
            commands.add_component(*entity, Selected);
        // If we're holding shift but haven't selected the unit, we need to check if we can add it
        // the current selection, because having a selection of a bunch of enemy units or a mixture
        // doesn't really make sense.
        } else {
            let only_player_units_selected = <&Side>::query()
                .filter(component::<Selected>())
                .iter(world)
                .all(|side| *side == player_side.0);

            if only_player_units_selected && *side == player_side.0 {
                commands.add_component(*entity, Selected);
            }
        }
    }
}

#[legion::system]
#[read_component(Side)]
#[write_component(CommandQueue)]
pub fn handle_right_click(
    #[resource] camera: &Camera,
    #[resource] mouse_state: &MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] rts_controls: &RtsControls,
    #[resource] player_side: &PlayerSide,
    world: &mut SubWorld,
) {
    if !mouse_state.right_state.was_clicked() {
        return;
    }

    let position = camera.cast_ray(mouse_state.position, screen_dimensions);

    <(&mut CommandQueue, &Side)>::query()
        .filter(component::<Selected>())
        .iter_mut(world)
        .filter(|(_, side)| **side == player_side.0)
        .for_each(|(commands, _)| {
            if !rts_controls.shift_held {
                commands.0.clear();
            }

            commands.0.push_back(Command::MoveTo(position));
        });
}

#[legion::system]
#[read_component(Side)]
#[write_component(CommandQueue)]
pub fn handle_stop_command(
    #[resource] rts_controls: &mut RtsControls,
    #[resource] player_side: &PlayerSide,
    world: &mut SubWorld,
) {
    if !rts_controls.s_pressed {
        return;
    }

    <(&mut CommandQueue, &Side)>::query()
        .filter(component::<Selected>())
        .iter_mut(world)
        .filter(|(_, side)| **side == player_side.0)
        .for_each(|(commands, _)| {
            commands.0.clear();
        });

    rts_controls.s_pressed = false;
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Side)]
#[read_component(Position)]
pub fn handle_drag_selection(
    #[resource] mouse_state: &MouseState,
    #[resource] camera: &Camera,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] rts_controls: &RtsControls,
    #[resource] player_side: &PlayerSide,
    command_buffer: &mut CommandBuffer,
    world: &SubWorld,
) {
    if let Some(start) = mouse_state.left_state.was_dragged() {
        let (top_left, bottom_right) = sort_points(start, mouse_state.position);
        let (left, right, top, bottom) = (top_left.x, bottom_right.x, top_left.y, bottom_right.y);

        if !rts_controls.shift_held {
            deselect_all(world, command_buffer);
        }

        <(Entity, &Position, &Side)>::query()
            .filter(component::<Selectable>())
            .iter(world)
            .filter(|(.., side)| **side == player_side.0)
            .for_each(|(entity, position, _)| {
                if point_is_in_select_box(
                    camera,
                    screen_dimensions,
                    position.0,
                    left,
                    right,
                    top,
                    bottom,
                ) {
                    command_buffer.add_component(*entity, Selected);
                }
            })
    }
}

#[legion::system]
pub fn update_mouse_buttons(#[resource] mouse_state: &mut MouseState) {
    let position = mouse_state.position;
    mouse_state.left_state.update(position);
    mouse_state.right_state.update(position);
}

fn deselect_all(world: &SubWorld, commands: &mut CommandBuffer) {
    <Entity>::query()
        .filter(component::<Selected>())
        .for_each(world, |entity| {
            commands.remove_component::<Selected>(*entity)
        });
}

fn point_is_in_select_box(
    camera: &Camera,
    screen_dimensions: &ScreenDimensions,
    point: Vec2,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
) -> bool {
    let point = vec2_to_ncollide_point(point);
    let top_left_point =
        vec2_to_ncollide_point(camera.cast_ray(Vec2::new(left, top), screen_dimensions));
    let top_right_point =
        vec2_to_ncollide_point(camera.cast_ray(Vec2::new(right, top), screen_dimensions));
    let bottom_left_point =
        vec2_to_ncollide_point(camera.cast_ray(Vec2::new(left, bottom), screen_dimensions));
    let bottom_right_point =
        vec2_to_ncollide_point(camera.cast_ray(Vec2::new(right, bottom), screen_dimensions));

    ncollide3d::utils::is_point_in_triangle(
        &point,
        &top_left_point,
        &top_right_point,
        &bottom_left_point,
    ) || ncollide3d::utils::is_point_in_triangle(
        &point,
        &top_right_point,
        &bottom_left_point,
        &bottom_right_point,
    )
}

fn vec2_to_ncollide_point(point: Vec2) -> ncollide3d::math::Point<f32> {
    ncollide3d::math::Point::new(point.x, 0.0, point.y)
}
