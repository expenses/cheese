use super::*;
use crate::resources::DeltaTime;

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
    damaged: &DamagedThisTick,
    health: &mut Health,
    // None in the case of a building.
    commands: Option<&mut CommandQueue>,
    map_handle: Option<&MapHandle>,
    buffer: &mut CommandBuffer,
    #[resource] map: &mut Map,
    world: &SubWorld,
) {
    health.0 = health.0.saturating_sub(2);

    if health.0 == 0 {
        buffer.remove(*entity);

        if let Some(map_handle) = map_handle {
            map.remove(map_handle);
        }

        return;
    }

    // If the unit is idle and got attacked, go attack back!
    if let Some(commands) = commands {
        let is_attaching_building = commands.0.front()
            .map(|command| match command {
                Command::Attack { target, explicit: false, .. } => {
                    <&Building>::query().get(world, *target).is_ok()
                },
                _ => false,
            }).unwrap_or(false);

        if commands.0.is_empty() || is_attaching_building {
            commands.0.push_front(Command::new_attack(damaged.0, false));
        }
    }

    buffer.remove_component::<DamagedThisTick>(*entity);
}

#[legion::system(for_each)]
#[filter(component::<Position>() & component::<Side>() & component::<FiringRange>())]
#[read_component(Entity)]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(FiringRange)]
pub fn add_attack_commands(entity: &Entity, commands: &mut CommandQueue, world: &SubWorld) {
    let (position, side, firing_range) = <(&Position, &Side, &FiringRange)>::query()
        .get(world, *entity)
        .expect("We've applied a filter for these components");

    let agro_multiplier = 1.5;

    if matches!(commands.0.front(), None | Some(&Command::MoveTo { attack_move: true, .. })) {
        let target = <(Entity, &Position, &Side)>::query()
            .iter(world)
            .filter(|(.., entity_side)| *entity_side != side)
            .filter(|(_, entity_position, _)| {
                (position.0 - entity_position.0).mag_sq()
                    <= (firing_range.0 * agro_multiplier).powi(2)
            })
            .min_by_key(|(_, entity_position, _)| {
                let distance_sq = (position.0 - entity_position.0).mag_sq();
                ordered_float::OrderedFloat(distance_sq)
            })
            .map(|(entity, ..)| entity);

        if let Some(target) = target {
            commands.0.push_front(Command::new_attack(*target, false))
        }
    }
}

#[legion::system(for_each)]
pub fn reduce_cooldowns(cooldown: &mut Cooldown, #[resource] delta_time: &DeltaTime) {
    cooldown.0 = (cooldown.0 - delta_time.0).max(0.0);
}
