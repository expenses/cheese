use super::*;
use crate::resources::{CommandMode, ControlGroups, DebugControls, RayCastLocation};

#[legion::system]
pub fn control_camera(
    #[resource] camera: &mut Camera,
    #[resource] camera_controls: &mut CameraControls,
    #[resource] mouse_state: &MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
) {
    let speed = 0.5;

    let edge_thickness = 50.0;
    let &ScreenDimensions {
        width: screen_width,
        height: screen_height,
    } = screen_dimensions;
    let screen_width = screen_width as f32;
    let screen_height = screen_height as f32;
    let mouse_x = mouse_state.position.x;
    let mouse_y = mouse_state.position.y;

    let right = Vec3::new(speed, 0.0, 0.0);
    let forwards = Vec3::new(0.0, 0.0, -speed);

    if camera_controls.left || mouse_x < edge_thickness {
        camera.position -= right;
        camera.looking_at -= right;
    }

    if camera_controls.right || mouse_x > screen_width - edge_thickness {
        camera.position += right;
        camera.looking_at += right;
    }

    if camera_controls.up || mouse_y < edge_thickness {
        camera.position += forwards;
        camera.looking_at += forwards;
    }

    if camera_controls.down || mouse_y > screen_height - edge_thickness {
        camera.position -= forwards;
        camera.looking_at -= forwards;
    }

    camera.position +=
        (camera.looking_at - camera.position).normalized() * camera_controls.zoom_delta * 0.01;
    camera_controls.zoom_delta = 0.0;
}

#[legion::system]
pub fn cast_ray(
    #[resource] camera: &Camera,
    #[resource] mouse_state: &MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] ray_cast_location: &mut RayCastLocation,
) {
    ray_cast_location.0 = camera.cast_ray(mouse_state.position, screen_dimensions);
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Selected)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Radius)]
#[write_component(CommandQueue)]
pub fn handle_left_click(
    #[resource] mouse_state: &MouseState,
    #[resource] ray_cast_location: &RayCastLocation,
    #[resource] rts_controls: &mut RtsControls,
    #[resource] player_side: &PlayerSide,
    world: &mut SubWorld,
    commands: &mut CommandBuffer,
) {
    if !mouse_state.left_state.was_clicked() {
        return;
    }

    match rts_controls.mode {
        CommandMode::AttackMove => {
            issue_command(ray_cast_location, rts_controls, player_side, world);
        }
        CommandMode::Normal => {
            let position = ray_cast_location.0;

            let entity = <(Entity, &Position, Option<&Selected>, &Side, &Radius)>::query()
                .filter(component::<Selectable>())
                .iter(world)
                .find(|(_, pos, .., radius)| (position - pos.0).mag_sq() < radius.0.powi(2))
                .map(|(entity, _, selected, side, _)| (entity, selected.is_some(), side));

            if !rts_controls.shift_held {
                deselect_all(world, commands);
            }

            if let Some((entity, is_selected, side)) = entity {
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
    }

    rts_controls.mode = CommandMode::Normal;
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Radius)]
#[write_component(CommandQueue)]
pub fn handle_right_click(
    #[resource] mouse_state: &MouseState,
    #[resource] ray_cast_location: &RayCastLocation,
    #[resource] rts_controls: &mut RtsControls,
    #[resource] player_side: &PlayerSide,
    world: &mut SubWorld,
) {
    if !mouse_state.right_state.was_clicked() {
        return;
    }

    // Copying SC2 here. If you're not in the normal command mode, attack moving, casting a spell,
    // whatever, then we want right clicks to just cancel that.
    if rts_controls.mode != CommandMode::Normal {
        rts_controls.mode = CommandMode::Normal;
        return;
    }

    issue_command(ray_cast_location, rts_controls, player_side, world)
}

fn issue_command(
    ray_cast_location: &RayCastLocation,
    rts_controls: &RtsControls,
    player_side: &PlayerSide,
    world: &mut SubWorld,
) {
    let position = ray_cast_location.0;

    let enemy_entity_under_cursor = <(Entity, &Position, &Side, &Radius)>::query()
        .iter(world)
        .filter(|(.., side, _)| **side != player_side.0)
        .find(|(_, pos, _, radius)| (position - pos.0).mag_sq() < radius.0.powi(2))
        .map(|(entity, ..)| entity);

    let command = match enemy_entity_under_cursor {
        Some(entity) => Command::Attack {
            target: *entity,
            explicit: true,
            first_out_of_range: true,
            out_of_range: true,
        },
        None => match rts_controls.mode {
            CommandMode::Normal => Command::MoveTo {
                target: position,
                attack_move: false,
            },
            CommandMode::AttackMove => Command::MoveTo {
                target: position,
                attack_move: true,
            },
        },
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
    #[resource] rts_controls: &RtsControls,
    #[resource] player_side: &PlayerSide,
    world: &mut SubWorld,
) {
    if !rts_controls.stop_pressed {
        return;
    }

    <(&mut CommandQueue, &Side)>::query()
        .filter(component::<Selected>())
        .iter_mut(world)
        .filter(|(_, side)| **side == player_side.0)
        .for_each(|(commands, _)| {
            commands.0.clear();
        });
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
#[read_component(Entity)]
pub fn remove_dead_entities_from_control_groups(
    #[resource] control_groups: &mut ControlGroups,
    world: &SubWorld,
) {
    for i in 0..10 {
        control_groups.0[i].retain(|entity| world.entry_ref(*entity).is_ok());
    }
}

#[legion::system]
#[read_component(Entity)]
pub fn handle_control_groups(
    #[resource] control_groups: &mut ControlGroups,
    #[resource] rts_controls: &RtsControls,
    command_buffer: &mut CommandBuffer,
    world: &SubWorld,
) {
    for i in 0..10 {
        if rts_controls.control_group_key_pressed[i] {
            if rts_controls.control_held {
                control_groups.0[i].clear();
                <Entity>::query()
                    .filter(component::<Selected>())
                    .for_each(world, |entity| {
                        control_groups.0[i].push(*entity);
                    });
            } else if rts_controls.shift_held {
                <Entity>::query()
                    .filter(component::<Selected>())
                    .for_each(world, |entity| {
                        control_groups.0[i].push(*entity);
                    });
            } else {
                if !control_groups.0[i].is_empty() {
                    deselect_all(world, command_buffer);
                }

                for entity in control_groups.0[i].iter() {
                    command_buffer.add_component(*entity, Selected);
                }
            }
        }
    }
}

#[legion::system]
pub fn cleanup_controls(
    #[resource] mouse_state: &mut MouseState,
    #[resource] rts_controls: &mut RtsControls,
    #[resource] debug_controls: &mut DebugControls,
) {
    let position = mouse_state.position;
    mouse_state.left_state.update(position);
    mouse_state.right_state.update(position);

    rts_controls.stop_pressed = false;

    for i in 0..10 {
        rts_controls.control_group_key_pressed[i] = false;
    }

    debug_controls.spawn_building_pressed = false;
    debug_controls.set_pathfinding_start_pressed = false;
}

fn deselect_all(world: &SubWorld, commands: &mut CommandBuffer) {
    <Entity>::query()
        .filter(component::<Selected>())
        .for_each(world, |entity| {
            commands.remove_component::<Selected>(*entity)
        });
}

#[test]
fn selection_and_deselection() {
    use crate::resources::*;

    let mut world = World::default();
    let mut resources = Resources::default();
    resources.insert(Camera::default());
    resources.insert(CameraControls::default());
    let screen_dimensions = ScreenDimensions {
        width: 1000,
        height: 1000,
    };
    resources.insert(MouseState::new(&screen_dimensions));
    resources.insert(screen_dimensions);
    resources.insert(RtsControls::default());
    resources.insert(PlayerSide(Side::Green));
    resources.insert(DeltaTime(1.0 / 60.0));
    resources.insert(RayCastLocation::default());
    resources.insert(ControlGroups::default());

    let mut builder = Schedule::builder();
    crate::add_gameplay_systems(&mut builder);
    let mut schedule = builder.build();
    let entity = Unit::MouseMarine.add_to_world(
        &mut world,
        None,
        Vec2::new(0.0, 0.0),
        Facing(0.0),
        Side::Green,
    );
    schedule.execute(&mut world, &mut resources);

    let mut query = <Option<&Selected>>::query();
    assert!(query.get(&world, entity).unwrap().is_none());

    {
        let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
        mouse_state.position = Vec2::new(500.0, 500.0);
        mouse_state.left_state = MouseButtonState::Clicked;
    }

    schedule.execute(&mut world, &mut resources);
    assert!(query.get(&world, entity).unwrap().is_some());

    {
        let mut mouse_state = resources.get_mut::<MouseState>().unwrap();
        mouse_state.position = Vec2::new(1000.0, 50.0);
        mouse_state.left_state = MouseButtonState::Clicked;
    }

    schedule.execute(&mut world, &mut resources);
    assert!(query.get(&world, entity).unwrap().is_none());
}
