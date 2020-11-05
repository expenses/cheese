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
#[read_component(Radius)]
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

    let entity = <(Entity, &Position, Option<&Selected>, &Side, &Radius)>::query()
        .filter(component::<Selectable>())
        .iter(world)
        .filter(|(_, pos, .., radius)| (position - pos.0).mag_sq() < radius.0.powi(2))
        //.min_by_key(|(_, pos)| (position - pos.0).mag_sq());
        .next()
        .map(|(entity, _, selected, side, _)| (entity, selected.is_some(), side));

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
#[read_component(Entity)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Radius)]
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

    let enemy_entity_under_cursor = <(Entity, &Position, &Side, &Radius)>::query()
        .iter(world)
        .filter(|(.., side, _)| **side != player_side.0)
        .filter(|(_, pos, _, radius)| (position - pos.0).mag_sq() < radius.0.powi(2))
        .next()
        .map(|(entity, ..)| entity);

    let command = match enemy_entity_under_cursor {
        Some(entity) => Command::Attack(*entity),
        None => Command::MoveTo(position),
    };

    <(&mut CommandQueue, &Side)>::query()
        .filter(component::<Selected>())
        .iter_mut(world)
        .filter(|(_, side)| **side == player_side.0)
        .for_each(|(commands, _)| {
            if !rts_controls.shift_held {
                commands.0.clear();
            }

            commands.0.push_back(command);
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
        let select_box = SelectBox::new(camera, screen_dimensions, start, mouse_state.position);

        if !rts_controls.shift_held {
            deselect_all(world, command_buffer);
        }

        <(Entity, &Position, &Side)>::query()
            .filter(component::<Selectable>())
            .iter(world)
            .filter(|(.., side)| **side == player_side.0)
            .for_each(|(entity, position, _)| {
                if select_box.contains(position.0) {
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
