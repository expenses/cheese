use super::*;

#[legion::system(for_each)]
pub fn stop_attacks_on_dead_entities(commands: &mut CommandQueue, world: &SubWorld) {
    while commands
        .0
        .front()
        .map(|command| {
            if let Command::Attack(entity) = command {
                world.entry_ref(*entity).is_err()
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
    command_queue: &CommandQueue,
    world: &SubWorld,
    buffer: &mut CommandBuffer,
    cooldown: &mut FiringCooldown,
) {
    if cooldown.0 != 0 {
        return;
    }

    let position = <&Position>::query()
        .get(world, *entity)
        .expect("We've applied a filter to this system for Position");

    if let Some(Command::Attack(target)) = command_queue.0.front() {
        let target_position = <&Position>::query()
            .get(world, *target)
            .expect("We've cancelled attack commands on dead entities");

        if (position.0 - target_position.0).mag_sq() <= FIRING_RANGE.powi(2) {
            buffer.push((
                Position(position.0),
                Bullet { target: *target },
                Facing(0.0),
                MoveTo(target_position.0),
            ));
            cooldown.0 = 10;
        }
    }
}

#[legion::system(for_each)]
#[write_component(Health)]
pub fn apply_bullets(
    entity: &Entity,
    bullet: &Bullet,
    position: &Position,
    move_to: &MoveTo,
    world: &mut SubWorld,
    buffer: &mut CommandBuffer,
) {
    if position.0 == move_to.0 {
        if let Ok(health) = <&mut Health>::query().get_mut(world, bullet.target) {
            health.0 = health.0.saturating_sub(1);
        }
        buffer.remove(*entity);
    }
}

#[legion::system(for_each)]
pub fn kill_dead(entity: &Entity, health: &Health, buffer: &mut CommandBuffer) {
    if health.0 == 0 {
        buffer.remove(*entity);
    }
}

#[legion::system(for_each)]
#[filter(component::<Position>() & component::<Side>())]
#[read_component(Entity)]
#[read_component(Position)]
#[read_component(Side)]
pub fn add_attack_commands(entity: &Entity, commands: &mut CommandQueue, world: &SubWorld) {
    let (position, side) = <(&Position, &Side)>::query()
        .get(world, *entity)
        .expect("We've applied a filter to this system for Position and Side");

    if matches!(commands.0.front().cloned(), None | Some(Command::AttackMove(_))) {
        let target = <(Entity, &Position, &Side)>::query()
            .iter(world)
            .filter(|(.., entity_side)| *entity_side != side)
            .filter(|(_, entity_position, _)| {
                (position.0 - entity_position.0).mag_sq() <= FIRING_RANGE.powi(2)
            })
            .next()
            .map(|(entity, ..)| entity);

        if let Some(target) = target {
            commands.0.push_front(Command::Attack(*target))
        }
    }
}

#[legion::system(for_each)]
pub fn reduce_cooldowns(cooldown: &mut FiringCooldown) {
    cooldown.0 = cooldown.0.saturating_sub(1);
}
