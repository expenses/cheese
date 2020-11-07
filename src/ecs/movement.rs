use super::*;
use crate::resources::DeltaTime;

#[legion::system(for_each)]
#[filter(component::<Position>())]
#[read_component(Position)]
pub fn set_move_to(
    entity: &Entity,
    firing_range: &FiringRange,
    commands: &mut CommandQueue,
    buffer: &mut CommandBuffer,
    world: &SubWorld,
) {
    // Grrrr.... In a `for_each` system, you can't pass in an `&T` and also have a query accessing
    // it, so we have to add `filter(component::<T>())` and do this.
    let position = <&Position>::query()
        .get(world, *entity)
        .expect("We've applied a filter to this system for Position");

    // Units try to get this much closer to enemies than their firing range.
    let fudge_factor = 0.05;

    let mut pop_front = false;

    match commands.0.front_mut() {
        Some(Command::MoveTo(target)) => buffer.add_component(*entity, MoveTo(*target)),
        Some(Command::AttackMove(target)) => buffer.add_component(*entity, MoveTo(*target)),
        Some(Command::Attack {
            target,
            first_out_of_range,
            ..
        }) => {
            let target_pos = <&Position>::query()
                .get(world, *target)
                .expect("We've cancelled attack commands on dead entities");

            let vector = target_pos.0 - position.0;

            let out_of_range = vector.mag_sq() > (firing_range.0 - fudge_factor).powi(2);

            // We check first_out_of_range here to make sure that units only chase when out of range
            // once.
            if out_of_range && *first_out_of_range {
                let mag = vector.mag();
                let distance_to_go = mag - (firing_range.0 - fudge_factor);
                let target = position.0 + vector.normalized() * distance_to_go;
                buffer.add_component(*entity, MoveTo(target));
            } else if out_of_range && !*first_out_of_range {
                // Remove the attack command so the unit can retarget.
                pop_front = true;
                buffer.remove_component::<MoveTo>(*entity);
            } else {
                *first_out_of_range = false;
                buffer.remove_component::<MoveTo>(*entity);
            }
        }
        None => buffer.remove_component::<MoveTo>(*entity),
    }

    if pop_front {
        commands.0.pop_front();
    }
}

#[legion::system(for_each)]
pub fn move_units(
    position: &mut Position,
    facing: &mut Facing,
    move_to: &MoveTo,
    move_speed: &MoveSpeed,
    // `None` if we're moving a bullet
    commands: Option<&mut CommandQueue>,
    #[resource] delta_time: &DeltaTime,
) {
    let direction = move_to.0 - position.0;
    if direction.mag_sq() > 0.0 {
        facing.0 = direction.y.atan2(direction.x);
    }

    let speed = move_speed.0 * delta_time.0;

    if direction.mag_sq() <= speed.powi(2) {
        position.0 = move_to.0;
        if let Some(commands) = commands {
            if commands
                .0
                .front()
                .map(|command| match command {
                    Command::MoveTo(_) | Command::AttackMove(_) => true,
                    Command::Attack { .. } => false
                })
                .unwrap_or(false)
            {
                commands.0.pop_front();
            }
        }
    } else {
        position.0 += direction.normalized() * speed;
    }
}

pub struct Avoidance(pub Vec2);
pub struct Avoids;
pub struct Avoidable;

#[legion::system]
#[read_component(Position)]
#[read_component(Radius)]
pub fn avoidance(world: &SubWorld, command_buffer: &mut CommandBuffer) {
    <(Entity, &Position, &Radius)>::query()
        .filter(component::<Avoids>())
        .for_each(world, |(entity, position, radius)| {
            let mut avoidance_direction = Vec2::new(0.0, 0.0);
            let mut count = 0;

            for (other_position, other_radius) in <(&Position, &Radius)>::query()
                .filter(component::<Avoidable>())
                .iter(world)
            {
                let away_vector = position.0 - other_position.0;
                let distance_sq = away_vector.mag_sq();
                let desired_seperation = radius.0 + other_radius.0;

                if distance_sq > 0.0 && distance_sq < desired_seperation.powi(2) {
                    let distance = distance_sq.sqrt();

                    avoidance_direction += away_vector.normalized() / distance;
                    count += 1;
                }
            }

            if count > 0 {
                avoidance_direction /= count as f32;
                command_buffer.add_component(*entity, Avoidance(avoidance_direction));
            }
        })
}

#[legion::system(for_each)]
pub fn apply_steering(
    entity: &Entity,
    position: &mut Position,
    avoidance: &Avoidance,
    command_buffer: &mut CommandBuffer,
) {
    // todo: delta time here
    position.0 += avoidance.0 * 0.1;
    command_buffer.remove_component::<Avoidance>(*entity);
}

#[legion::system(for_each)]
#[read_component(Position)]
pub fn set_move_to_for_bullets(
    entity: &Entity,
    bullet: &Bullet,
    world: &SubWorld,
    buffer: &mut CommandBuffer,
) {
    if let Ok(target_position) = <&Position>::query().get(world, bullet.target) {
        buffer.add_component(*entity, MoveTo(target_position.0));
    }
}
