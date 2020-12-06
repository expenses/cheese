use super::*;
use crate::resources::{DeltaTime, GameStats, PlayerSide};

#[legion::system(for_each)]
#[read_component(Position)]
pub fn stop_actions_on_dead_entities(commands: &mut CommandQueue, world: &SubWorld) {
    while commands
        .0
        .front()
        .map(|command| {
            if let Command::Attack { target, .. } | Command::Build { target, .. } = command {
                world.entry_ref(*target).is_err()
            } else {
                false
            }
        })
        .unwrap_or(false)
    {
        commands.0.pop_front();
    }
}

#[legion::system(for_each)]
#[filter(component::<Position>())]
#[read_component(Position)]
pub fn firing(
    entity: &Entity,
    facing: &mut Facing,
    cooldown: &mut Cooldown,
    firing_range: &FiringRange,
    command_queue: &CommandQueue,
    world: &SubWorld,
    buffer: &mut CommandBuffer,
) {
    if cooldown.0 != 0.0 {
        return;
    }

    let position = <&Position>::query()
        .get(world, *entity)
        .expect("We've applied a filter to this system for Position");

    if let Some(Command::Attack { target, .. }) = command_queue.0.front() {
        let target_position = <&Position>::query()
            .get(world, *target)
            .expect("We've cancelled actions on dead entities");

        let vector = target_position.0 - position.0;

        if vector.mag_sq() <= firing_range.0.powi(2) {
            facing.0 = vector.y.atan2(vector.x);

            buffer.push((
                Position(position.0 + vector.normalized() * 0.5),
                Bullet {
                    target: *target,
                    source: *entity,
                    target_position: target_position.0,
                },
                Facing(vector.y.atan2(vector.x)),
                MoveSpeed(20.0),
            ));
            cooldown.0 = 10.0 / 60.0;
        }
    }
}

#[legion::system(for_each)]
// Need this so that entry_ref works instead of erroring for a non-obvious reason.
#[read_component(Entity)]
pub fn apply_bullets(
    entity: &Entity,
    bullet: &Bullet,
    position: &Position,
    world: &SubWorld,
    buffer: &mut CommandBuffer,
) {
    if position.0 == bullet.target_position {
        if world.entry_ref(bullet.target).is_ok() {
            buffer.add_component(bullet.target, DamagedThisTick(bullet.source));
        }
        buffer.remove(*entity);
    }
}

#[legion::system(for_each)]
#[read_component(Building)]
pub fn handle_damaged(
    entity: &Entity,
    position: &Position,
    radius: &Radius,
    damaged: &DamagedThisTick,
    side: &Side,
    health: &mut Health,
    // None in the case of a building.
    commands: Option<&mut CommandQueue>,
    can_attack: Option<&CanAttack>,
    map_handle: Option<&MapHandle>,
    buffer: &mut CommandBuffer,
    #[resource] player_side: &PlayerSide,
    #[resource] stats: &mut GameStats,
    #[resource] map: &mut Map,
    #[resource] rng: &mut SmallRng,
    world: &SubWorld,
) {
    health.0 = (health.0 - 2.0).max(0.0);

    if health.0 == 0.0 {
        buffer.remove(*entity);

        if let Some(map_handle) = map_handle {
            map.remove(map_handle);
        }

        if *side == player_side.0 {
            stats.units_lost += 1;
        } else if map_handle.is_some() {
            stats.enemy_buildings_destroyed += 1;
        } else {
            stats.enemy_units_killed += 1;
        }

        buffer.push((Explosion::new(position.0, rng, radius.0),));

        return;
    }

    // If the unit is idle and got attacked, go attack back!
    if let Some(commands) = commands {
        if can_attack.is_some()
            && (commands.0.is_empty() || is_attacking_building(&commands, world))
        {
            commands.0.push_front(Command::new_attack(damaged.0, false));
        }
    }

    buffer.remove_component::<DamagedThisTick>(*entity);
}

fn is_attacking_building(commands: &CommandQueue, world: &SubWorld) -> bool {
    commands
        .0
        .front()
        .map(|command| match command {
            Command::Attack {
                target,
                explicit: false,
                ..
            } => <&Building>::query().get(world, *target).is_ok(),
            _ => false,
        })
        .unwrap_or(false)
}

#[legion::system(for_each)]
#[filter(component::<Position>() & component::<Side>() & component::<CanAttack>())]
#[read_component(Entity)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Building)]
pub fn agro_units(
    entity: &Entity,
    commands: &mut CommandQueue,
    world: &SubWorld,
    command_buffer: &mut CommandBuffer,
) {
    // Todo: find a clean way to getting units to re-target when an enemy unit is in range and we're
    // currently attacking a building.

    let is_available_to_attack =
        matches!(commands.0.front(), None | Some(&Command::MoveTo { attack_move: true, .. }));

    if !is_available_to_attack {
        return;
    }

    let (position, side) = <(&Position, &Side)>::query()
        .get(world, *entity)
        .expect("We've applied a filter for these components");

    let agro_range: f32 = 15.0;

    if let Some(target) = find_best_target(position.0, *side, Some(agro_range), world) {
        commands.0.push_front(Command::new_attack(target, false));
        command_buffer.add_component(*entity, Agroed::ThisTick(target));
    }
}

fn find_best_target(
    position: Vec2,
    side: Side,
    in_range: Option<f32>,
    world: &SubWorld,
) -> Option<Entity> {
    <(Entity, &Position, Option<&Building>, &Side)>::query()
        .iter(world)
        .filter(|(.., entity_side)| **entity_side != side)
        .filter(|(_, entity_position, ..)| {
            in_range
                .map(|range| (position - entity_position.0).mag_sq() <= range.powi(2))
                .unwrap_or(true)
        })
        .map(|(entity, entity_position, entity_building, _)| {
            let distance_sq = (position - entity_position.0).mag_sq();
            (
                *entity,
                ordered_float::OrderedFloat(distance_sq),
                entity_building.is_some(),
            )
        })
        .min_by(|&(_, a_pos, a_is_building), &(_, b_pos, b_is_building)| {
            if a_is_building == b_is_building {
                a_pos.cmp(&b_pos)
            // If only a is a building, then it a has less priority
            } else if a_is_building {
                std::cmp::Ordering::Greater
            // If only b is a building then it a has higher priority
            } else {
                std::cmp::Ordering::Less
            }
        })
        .map(|(entity, ..)| entity)
}

#[legion::system(for_each)]
#[filter(component::<Position>() & component::<Side>() & component::<CanAttack>())]
#[read_component(Entity)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Building)]
#[read_component(Agroed)]
pub fn propagate_agro(
    entity: &Entity,
    commands: &mut CommandQueue,
    world: &SubWorld,
    command_buffer: &mut CommandBuffer,
) {
    let is_available_to_attack =
        matches!(commands.0.front(), None | Some(&Command::MoveTo { attack_move: true, .. }));

    if !is_available_to_attack {
        return;
    }

    let (position, side) = <(&Position, &Side)>::query()
        .get(world, *entity)
        .expect("We've applied a filter for these components");

    let agro_propagation_distance: f32 = 5.0;

    let agro_entity = <(&Position, &Side, &Agroed)>::query()
        .iter(world)
        .filter(|(unit_pos, unit_side, _)| {
            *unit_side == side
                && (unit_pos.0 - position.0).mag_sq() <= agro_propagation_distance.powi(2)
        })
        .next()
        .map(|(.., agroed)| match agroed {
            Agroed::ThisTick(entity) => *entity,
            Agroed::LastTick(entity) => *entity,
        });

    if let Some(target) = agro_entity {
        commands.0.push_front(Command::new_attack(target, false));
        command_buffer.add_component(*entity, Agroed::ThisTick(target));
    }
}

#[legion::system(for_each)]
pub fn update_argoed_this_tick(entity: &Entity, agroed: &mut Agroed, buffer: &mut CommandBuffer) {
    match *agroed {
        Agroed::ThisTick(entity) => *agroed = Agroed::LastTick(entity),
        Agroed::LastTick(_) => buffer.remove_component::<Agroed>(*entity),
    }
}

#[legion::system(for_each)]
pub fn reduce_cooldowns(cooldown: &mut Cooldown, #[resource] delta_time: &DeltaTime) {
    cooldown.0 = (cooldown.0 - delta_time.0).max(0.0);
}
