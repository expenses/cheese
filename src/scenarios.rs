use crate::assets::ModelAnimations;
use crate::ecs;
use crate::pathfinding::Map;
use crate::resources::{Camera, CheeseCoins, LoseCondition, Objectives, WinCondition};
use legion::systems::CommandBuffer;
use legion::*;
use rand::Rng;
use ultraviolet::Vec2;

// Squad of 10 marines vs 5.
pub fn one(
    world: &mut World,
    animations: &ModelAnimations,
    map: &mut Map,
    rng: &mut rand::rngs::SmallRng,
    objectives: &mut Objectives,
    camera: &mut Camera,
    cheese_coins: &mut CheeseCoins,
) {
    world.clear();

    let mut command_buffer = legion::systems::CommandBuffer::new(&world);

    let unit_spawn_point = Vec2::new(-36.0, 0.0);

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        10,
        unit_spawn_point,
        2.0,
        0.0,
        ecs::Side::Green,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        5,
        Vec2::new(40.0, 6.7),
        1.0,
        180.0,
        ecs::Side::Purple,
    );

    command_buffer.flush(world);

    world.push((ecs::Explosion::new(Vec2::zero(), rng),));

    ecs::Building::Armoury
        .add_to_world_fully_built(
            world,
            Vec2::new(36.0, -10.0),
            ecs::Side::Purple,
            animations,
            map,
        )
        .unwrap();

    spawn_pump_over_guyser(
        Vec2::new(46.0, -5.0),
        ecs::Side::Purple,
        world,
        animations,
        map,
        rng,
    );

    ecs::Building::Armoury
        .add_to_world_fully_built(
            world,
            Vec2::new(54.0, 2.0),
            ecs::Side::Purple,
            animations,
            map,
        )
        .unwrap();

    ecs::Building::Armoury
        .add_to_world_fully_built(
            world,
            Vec2::new(44.0, 16.0),
            ecs::Side::Purple,
            animations,
            map,
        )
        .unwrap();

    spawn_pump_over_guyser(
        Vec2::new(54.0, 19.0),
        ecs::Side::Purple,
        world,
        animations,
        map,
        rng,
    );

    spawn_pump_over_guyser(
        Vec2::new(36.0, 22.0),
        ecs::Side::Purple,
        world,
        animations,
        map,
        rng,
    );

    *objectives = Objectives {
        win_conditions: vec![WinCondition::DestroyAll],
        lose_conditions: vec![LoseCondition::LetAllUnitsDie],
    };

    *camera = Camera {
        looking_at: unit_spawn_point,
        distance: 15.0,
    };
    *cheese_coins = CheeseCoins(0);
}

fn spawn_guyser(world: &mut World, position: Vec2) {
    world.push((
        ecs::Position(position),
        ecs::CheeseGuyser,
        ecs::Cooldown(0.0),
    ));
}

fn spawn_pump_over_guyser(
    position: Vec2,
    side: ecs::Side,
    world: &mut World,
    animations: &ModelAnimations,
    map: &mut Map,
    rng: &mut rand::rngs::SmallRng,
) {
    let pump_entity = ecs::Building::Pump
        .add_to_world_fully_built(world, position, side, animations, map)
        .unwrap();

    let animation_offset = rng.gen_range(0.0, 1.0);

    <&mut ecs::AnimationState>::query()
        .get_mut(world, pump_entity)
        .unwrap()
        .time = animation_offset;

    world.push((
        ecs::Position(position),
        ecs::CheeseGuyser,
        ecs::CheeseGuyserBuiltOn { pump: pump_entity },
        ecs::Cooldown(0.0),
    ));
}

fn spawn_units_in_circle(
    buffer: &mut CommandBuffer,
    animations: &ModelAnimations,
    num: u32,
    center: Vec2,
    radius: f32,
    facing: f32,
    side: ecs::Side,
) {
    for i in 0..num {
        let rads = (i as f32 / num as f32 * 360.0).to_radians();
        ecs::Unit::MouseMarine.add_to_world(
            buffer,
            Some(animations),
            center + Vec2::new(rads.sin() * radius, rads.cos() * radius),
            ecs::Facing(facing.to_radians()),
            side,
            None,
        );
    }
}

// Single engineer has to build a base.
pub fn two(
    mut world: &mut World,
    animations: &ModelAnimations,
    map: &mut Map,
    rng: &mut rand::rngs::SmallRng,
    objectives: &mut Objectives,
    camera: &mut Camera,
    cheese_coins: &mut CheeseCoins,
) {
    let engineer_pos = Vec2::new(-52.69, -53.42);

    world.clear();

    let mut command_buffer = legion::systems::CommandBuffer::new(&world);

    ecs::Unit::Engineer.add_to_world(
        &mut command_buffer,
        Some(animations),
        engineer_pos,
        ecs::Facing(-1.0),
        ecs::Side::Green,
        None,
    );

    let enemy_pos = Vec2::new(15.36, 66.0);

    let direction = engineer_pos - enemy_pos;

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        5,
        enemy_pos + Vec2::new(-3.0, 0.0),
        1.0,
        direction.y.atan2(direction.x).to_degrees(),
        ecs::Side::Purple,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        5,
        enemy_pos + Vec2::new(3.0, 0.0),
        1.0,
        direction.y.atan2(direction.x).to_degrees(),
        ecs::Side::Purple,
    );

    command_buffer.flush(world);

    // Spawn guysers

    spawn_guyser(&mut world, Vec2::new(-69.58, -78.49));
    spawn_guyser(&mut world, Vec2::new(-59.77, -68.08));
    spawn_guyser(&mut world, Vec2::new(-71.5, -39.26));
    spawn_guyser(&mut world, Vec2::new(-27.52, -59.68));
    spawn_guyser(&mut world, Vec2::new(-42.69, -77.58));

    *camera = Camera {
        looking_at: engineer_pos,
        distance: 30.0,
    };

    *objectives = Objectives {
        win_conditions: vec![
            WinCondition::DestroyAll,
            WinCondition::BuildN(2, ecs::Building::Pump),
            WinCondition::BuildN(1, ecs::Building::Armoury),
        ],
        lose_conditions: vec![LoseCondition::LetAllUnitsDie],
    };
    *cheese_coins = CheeseCoins(100);
}
