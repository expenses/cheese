use super::*;
use crate::assets::ModelAnimations;
use crate::resources::{
    CheeseCoins, CommandMode, ControlGroups, Keypress, Keypresses, LoseCondition, Mode, Objectives,
    RayCastLocation, SelectedUnitsAbilities, WinCondition,
};

#[legion::system]
#[write_component(RecruitmentQueue)]
pub fn handle_keypresses(
    #[resource] keypresses: &mut Keypresses,
    #[resource] camera_controls: &mut CameraControls,
    #[resource] rts_controls: &mut RtsControls,
    #[resource] debug_controls: &mut DebugControls,
    #[resource] cheese_coins: &mut CheeseCoins,
    #[resource] player_side: &mut PlayerSide,
    #[resource] selected_units_abilities: &SelectedUnitsAbilities,
    #[resource] mode: &mut Mode,
    world: &mut SubWorld,
) {
    for Keypress {
        code,
        scancode,
        pressed,
    } in keypresses.0.drain(..)
    {
        log::trace!("{:?} (scancode: {}) pressed: {}", code, scancode, pressed);

        if let Some(code) = code {
            if pressed {
                for (ability, casters) in selected_units_abilities.0.iter() {
                    if code == ability.hotkey {
                        match ability.ability_type {
                            AbilityType::SetRecruitmentWaypoint => {
                                rts_controls.mode = CommandMode::SetRecruitmentWaypoint;
                            }
                            AbilityType::Build(building) => {
                                //if building.stats().cost <= cheese_coins.0 {
                                rts_controls.mode = CommandMode::Construct { building };
                                //} else {
                                // Todo: play sound: meep merp (like from dota).
                                //}
                            }
                            AbilityType::Recruit(unit) => {
                                if unit.stats().cost <= cheese_coins.0 {
                                    cheese_coins.0 -= unit.stats().cost;

                                    let entity_with_shortest_recruitment_queue = casters
                                        .iter()
                                        .map(|caster| {
                                            let queue_len = <&RecruitmentQueue>::query()
                                                .get(world, *caster)
                                                .unwrap()
                                                .queue
                                                .len();
                                            (caster, queue_len)
                                        })
                                        .min_by_key(|(_, queue_len)| *queue_len)
                                        .map(|(entity, _)| *entity)
                                        .unwrap();

                                    <&mut RecruitmentQueue>::query()
                                        .get_mut(world, entity_with_shortest_recruitment_queue)
                                        .unwrap()
                                        .queue
                                        .push_back(unit);
                                }
                            }
                        }
                    }
                }
            }

            match code {
                //VirtualKeyCode::X if pressed => player_side.0 = Side::Purple,
                VirtualKeyCode::Up => camera_controls.up = pressed,
                VirtualKeyCode::Down => camera_controls.down = pressed,
                VirtualKeyCode::Left => camera_controls.left = pressed,
                VirtualKeyCode::Right => camera_controls.right = pressed,
                VirtualKeyCode::LShift => rts_controls.shift_held = pressed,
                VirtualKeyCode::LControl => rts_controls.control_held = pressed,
                VirtualKeyCode::S if pressed => rts_controls.stop_pressed = true,
                VirtualKeyCode::A if pressed => rts_controls.mode = CommandMode::AttackMove,
                VirtualKeyCode::T if pressed => debug_controls.set_pathfinding_start_pressed = true,
                VirtualKeyCode::Escape if pressed => {
                    if rts_controls.mode != CommandMode::Normal {
                        rts_controls.mode = CommandMode::Normal;
                    } else {
                        *mode = Mode::PlayingMenu;
                    }
                }

                VirtualKeyCode::Key0 if pressed => rts_controls.control_group_key_pressed[0] = true,
                VirtualKeyCode::Key1 if pressed => rts_controls.control_group_key_pressed[1] = true,
                VirtualKeyCode::Key2 if pressed => rts_controls.control_group_key_pressed[2] = true,
                VirtualKeyCode::Key3 if pressed => rts_controls.control_group_key_pressed[3] = true,
                VirtualKeyCode::Key4 if pressed => rts_controls.control_group_key_pressed[4] = true,
                VirtualKeyCode::Key5 if pressed => rts_controls.control_group_key_pressed[5] = true,
                VirtualKeyCode::Key6 if pressed => rts_controls.control_group_key_pressed[6] = true,
                VirtualKeyCode::Key7 if pressed => rts_controls.control_group_key_pressed[7] = true,
                VirtualKeyCode::Key8 if pressed => rts_controls.control_group_key_pressed[8] = true,
                VirtualKeyCode::Key9 if pressed => rts_controls.control_group_key_pressed[9] = true,

                _ => {}
            }
        }

        // Pressing shift + a number key doesn't output a virtualkeycode so we have to use scancodes instead.
        match scancode {
            11 if pressed => rts_controls.control_group_key_pressed[0] = true,
            2 if pressed => rts_controls.control_group_key_pressed[1] = true,
            3 if pressed => rts_controls.control_group_key_pressed[2] = true,
            4 if pressed => rts_controls.control_group_key_pressed[3] = true,
            5 if pressed => rts_controls.control_group_key_pressed[4] = true,
            6 if pressed => rts_controls.control_group_key_pressed[5] = true,
            7 if pressed => rts_controls.control_group_key_pressed[6] = true,
            8 if pressed => rts_controls.control_group_key_pressed[7] = true,
            9 if pressed => rts_controls.control_group_key_pressed[8] = true,
            10 if pressed => rts_controls.control_group_key_pressed[9] = true,
            _ => {}
        }
    }
}

#[legion::system]
pub fn control_camera(
    #[resource] camera: &mut Camera,
    #[resource] camera_controls: &mut CameraControls,
    #[resource] mouse_state: &MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] delta_time: &DeltaTime,
) {
    let speed = 45.0 * delta_time.0;

    let edge_thickness = 50.0;
    let &ScreenDimensions {
        width: screen_width,
        height: screen_height,
    } = screen_dimensions;
    let screen_width = screen_width as f32;
    let screen_height = screen_height as f32;
    let mouse_x = mouse_state.position.x;
    let mouse_y = mouse_state.position.y;

    let right = Vec2::new(speed, 0.0);
    let forwards = Vec2::new(0.0, -speed);

    if camera_controls.left || mouse_x < edge_thickness {
        camera.looking_at -= right;
    }

    if camera_controls.right || mouse_x > screen_width - edge_thickness {
        camera.looking_at += right;
    }

    if camera_controls.up || mouse_y < edge_thickness {
        camera.looking_at += forwards;
    }

    if camera_controls.down || mouse_y > screen_height - edge_thickness {
        camera.looking_at -= forwards;
    }

    camera.looking_at.x = camera.looking_at.x.min(100.0).max(-100.0);
    camera.looking_at.y = camera.looking_at.y.min(100.0).max(-100.0);

    camera.distance = (camera.distance - camera_controls.zoom_delta * 0.01)
        .max(5.0)
        .min(90.0);
    camera_controls.zoom_delta = 0.0;
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Position)]
pub fn cast_ray(
    #[resource] camera: &Camera,
    #[resource] mouse_state: &MouseState,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] ray_cast_location: &mut RayCastLocation,
    #[resource] rts_controls: &RtsControls,
    world: &SubWorld,
) {
    ray_cast_location.pos = camera.cast_ray(mouse_state.position, screen_dimensions);
    ray_cast_location.snapped_to_guyser = None;
    if let CommandMode::Construct {
        building: Building::Pump,
    } = rts_controls.mode
    {
        let snap_guyser = <(Entity, &Position)>::query()
            .filter(component::<CheeseGuyser>() & !component::<CheeseGuyserBuiltOn>())
            .iter(world)
            .find(|(_, pos)| (ray_cast_location.pos - pos.0).mag_sq() <= 4.0_f32.powi(2));

        if let Some((entity, pos)) = snap_guyser {
            ray_cast_location.pos = pos.0;
            ray_cast_location.snapped_to_guyser = Some(*entity);
        }
    }
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Selected)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Radius)]
#[read_component(Building)]
#[write_component(CommandQueue)]
#[write_component(RecruitmentQueue)]
pub fn handle_left_click(
    #[resource] mouse_state: &MouseState,
    #[resource] ray_cast_location: &RayCastLocation,
    #[resource] rts_controls: &mut RtsControls,
    #[resource] player_side: &PlayerSide,
    #[resource] map: &mut Map,
    #[resource] animations: &ModelAnimations,
    #[resource] cheese_coins: &mut CheeseCoins,
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
            let position = ray_cast_location.pos;

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
        CommandMode::Construct { building } => {
            build_building_command(
                building,
                ray_cast_location,
                player_side,
                map,
                animations,
                commands,
                world,
                rts_controls,
                cheese_coins,
            );
        }
        CommandMode::SetRecruitmentWaypoint => {
            let position = ray_cast_location.pos;

            <(&mut RecruitmentQueue, &Side)>::query()
                .filter(component::<Selected>())
                .iter_mut(world)
                .filter(|(_, side)| **side == player_side.0)
                .for_each(|(queue, _)| {
                    queue.waypoint = position;
                })
        }
    }

    if !rts_controls.shift_held {
        rts_controls.mode = CommandMode::Normal;
    }
}

fn build_building_command(
    building: Building,
    ray_cast_location: &RayCastLocation,
    player_side: &PlayerSide,
    map: &mut Map,
    animations: &ModelAnimations,
    commands: &mut CommandBuffer,
    world: &mut SubWorld,
    rts_controls: &RtsControls,
    cheese_coins: &mut CheeseCoins,
) {
    if building.stats().cost > cheese_coins.0
        || (building == Building::Pump && ray_cast_location.snapped_to_guyser.is_none())
    {
        return;
    }

    if let Some((pos, handle, building, radius, selectable, side, health, completeness)) =
        building.parts(ray_cast_location.pos, player_side.0, map)
    {
        cheese_coins.0 -= building.stats().cost;

        let building_entity = match building {
            Building::Pump => {
                let skin = animations.pump.skin.clone();
                let animation_state = AnimationState {
                    animation: 0,
                    time: 0.0,
                    total_time: animations.pump.animations[0].total_time,
                };
                commands.push((
                    pos,
                    handle,
                    building,
                    radius,
                    selectable,
                    side,
                    health,
                    skin,
                    animation_state,
                    completeness,
                    Cooldown(0.0),
                ))
            }
            Building::Armoury => commands.push((
                pos,
                handle,
                building,
                radius,
                selectable,
                side,
                health,
                completeness,
                Abilities(vec![
                    &Ability::RECRUIT_MOUSE_MARINE,
                    &Ability::RECRUIT_ENGINEER,
                    &Ability::SET_RECRUITMENT_WAYPOINT,
                ]),
                RecruitmentQueue::default(),
                Cooldown(0.0),
            )),
        };

        if let Building::Pump = building {
            let guyser_entity = ray_cast_location.snapped_to_guyser.unwrap();
            commands.add_component(
                guyser_entity,
                CheeseGuyserBuiltOn {
                    pump: building_entity,
                },
            );
        }

        let command = Command::Build {
            target: building_entity,
            // Kinda hacky? If we put `ActionState::OutOfRange` with an empty vec it
            // wouldn't get updated with the current `set_movement_paths` code.
            state: ActionState::InRange,
        };

        <(&mut CommandQueue, &Side)>::query()
            .filter(component::<Selected>() & component::<CanBuild>())
            .iter_mut(world)
            .filter(|(_, side)| **side == player_side.0)
            .for_each(|(commands, _)| {
                if !rts_controls.shift_held {
                    commands.0.clear();
                }

                commands.0.push_back(command.clone());
            });
    }
}

#[legion::system]
#[read_component(Entity)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Radius)]
#[read_component(Building)]
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
    let position = ray_cast_location.pos;

    let entity_under_cursor = <(Entity, &Position, &Side, &Radius, Option<&Building>)>::query()
        .iter(world)
        .find(|(_, pos, _, radius, _)| (position - pos.0).mag_sq() < radius.0.powi(2))
        .map(|(entity, _, side, .., building)| {
            (*entity, *side == player_side.0, building.is_some())
        });

    let command = match entity_under_cursor {
        Some((entity, false, _)) => Some(Command::new_attack(entity, true)),
        Some((entity, true, true)) => Some(Command::Build {
            target: entity,
            state: ActionState::InRange,
        }),
        Some((_, true, false)) => None,
        None => match rts_controls.mode {
            CommandMode::Normal => Some(Command::MoveTo {
                target: position,
                path: Vec::new(),
                attack_move: false,
            }),
            CommandMode::AttackMove => Some(Command::MoveTo {
                target: position,
                path: Vec::new(),
                attack_move: true,
            }),
            CommandMode::Construct { .. } => None,
            CommandMode::SetRecruitmentWaypoint => None,
        },
    };

    if let Some(command) = command {
        if let Command::Build { .. } = command {
            <(&mut CommandQueue, &Side)>::query()
                .filter(component::<Selected>() & component::<CanBuild>())
                .iter_mut(world)
                .filter(|(_, side)| **side == player_side.0)
                .for_each(|(commands, _)| {
                    if !rts_controls.shift_held {
                        commands.0.clear();
                    }

                    commands.0.push_back(command.clone());
                });
        } else {
            <(&mut CommandQueue, &Side)>::query()
                .filter(component::<Selected>())
                .iter_mut(world)
                .filter(|(_, side)| **side == player_side.0)
                .for_each(|(commands, _)| {
                    if !rts_controls.shift_held {
                        commands.0.clear();
                    }

                    commands.0.push_back(command.clone());
                });
        }
    } else {
        log::debug!("Ignoring command on {:?}", entity_under_cursor);
    }
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

                for entity in control_groups.0[i].iter() {
                    command_buffer.add_component(*entity, Selected);
                }
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
#[read_component(Abilities)]
#[read_component(Side)]
pub fn update_selected_units_abilities(
    #[resource] player_side: &PlayerSide,
    #[resource] selected_units_abilities: &mut SelectedUnitsAbilities,
    world: &SubWorld,
) {
    selected_units_abilities.0.clear();

    <(Entity, &Abilities, &Side)>::query()
        .filter(component::<Selected>())
        .iter(world)
        .filter(|(.., side)| **side == player_side.0)
        .flat_map(|(entity, abilities, _)| {
            abilities.0.iter().map(move |ability| (entity, *ability))
        })
        .for_each(|(entity, ability)| {
            selected_units_abilities
                .0
                .entry(ability)
                .or_insert_with(Vec::new)
                .push(*entity);
        });
}

fn deselect_all(world: &SubWorld, commands: &mut CommandBuffer) {
    <Entity>::query()
        .filter(component::<Selected>())
        .for_each(world, |entity| {
            commands.remove_component::<Selected>(*entity)
        });
}

#[legion::system]
#[read_component(Side)]
#[read_component(Building)]
#[read_component(BuildingCompleteness)]
pub fn update_playing_state(
    #[resource] objectives: &Objectives,
    #[resource] player_side: &PlayerSide,
    #[resource] mode: &mut Mode,
    world: &SubWorld,
) {
    let won = objectives
        .win_conditions
        .iter()
        .all(|condition| match condition {
            WinCondition::DestroyAll => {
                let all_destroyed = <&Side>::query()
                    .iter(world)
                    .all(|side| *side == player_side.0);
                all_destroyed
            }
            WinCondition::BuildN(num, building) => {
                let num_buildings = <(&Side, &Building, &BuildingCompleteness)>::query()
                    .iter(world)
                    .filter(|(side, building_type, completeness)| {
                        **side == player_side.0
                            && building == *building_type
                            && completeness.0 == building.stats().max_health
                    })
                    .count();
                num_buildings as u8 >= *num
            }
        });

    if won {
        *mode = Mode::ScenarioWon;
        return;
    }

    let lost = objectives
        .lose_conditions
        .iter()
        .any(|condition| match condition {
            LoseCondition::LetAllUnitsDie => {
                let all_units_dead = <&Side>::query()
                    .filter(component::<Unit>())
                    .iter(world)
                    .all(|side| *side != player_side.0);

                all_units_dead
            }
        });

    if lost {
        *mode = Mode::ScenarioLost;
    }
}

#[test]
fn selection_and_deselection() {
    use crate::assets::ModelAnimations;
    use crate::pathfinding::Map;
    use crate::resources::*;
    use rand::SeedableRng;

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
    resources.insert(Map::new());
    resources.insert(Gravity(5.0));
    resources.insert(DebugControls::default());
    resources.insert(rand::rngs::SmallRng::from_entropy());
    resources.insert(ModelAnimations::default());

    let mut builder = Schedule::builder();
    super::add_gameplay_systems(&mut builder);
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
