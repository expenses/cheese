use crate::assets::ModelAnimations;
use crate::ecs;
use crate::pathfinding::Map;
use legion::systems::CommandBuffer;
use legion::*;
use ultraviolet::Vec2;
use rand::Rng;

// Squad of 10 marines vs 5.
pub fn one(world: &mut World, animations: &ModelAnimations, map: &mut Map, rng: &mut rand::rngs::SmallRng) {
    world.clear();

    let mut command_buffer = legion::systems::CommandBuffer::new(&world);

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        10,
        Vec2::new(-36.0, 0.0),
        0.0,
        ecs::Side::Green,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        5,
        Vec2::new(40.0, 6.7),
        180.0,
        ecs::Side::Purple,
    );

    command_buffer.flush(world);

    ecs::Building::Armoury.add_to_world_fully_built(
        world,
        Vec2::new(36.0, -10.0),
        ecs::Side::Purple,
        animations,
        map,
    ).unwrap();

    spawn_pump_over_guyser(
        Vec2::new(46.0, -5.0),
        ecs::Side::Purple,
        world,
        animations, map, rng
    );

    ecs::Building::Armoury.add_to_world_fully_built(
        world,
        Vec2::new(54.0, 2.0),
        ecs::Side::Purple,
        animations,
        map,
    ).unwrap();

    ecs::Building::Armoury.add_to_world_fully_built(
        world,
        Vec2::new(44.0, 16.0),
        ecs::Side::Purple,
        animations,
        map,
    ).unwrap();

    spawn_pump_over_guyser(
        Vec2::new(54.0, 19.0),
        ecs::Side::Purple,
        world,
        animations, map, rng,
    );

    spawn_pump_over_guyser(
        Vec2::new(36.0, 22.0),
        ecs::Side::Purple,
        world,
        animations, map, rng,
    );
}

fn spawn_pump_over_guyser(
    position: Vec2,
    side: ecs::Side,
    world: &mut World,
    animations: &ModelAnimations,
    map: &mut Map,
    rng: &mut rand::rngs::SmallRng,
) {
    let pump_entity = ecs::Building::Pump.add_to_world_fully_built(
        world,
        position,
        side,
        animations,
        map,
    ).unwrap();

    let animation_offset = rng.gen_range(0.0, 1.0);

    <&mut ecs::AnimationState>::query().get_mut(world, pump_entity)
        .unwrap().time = animation_offset;

    world.push((
        ecs::Position(position),
        ecs::CheeseGuyser,
        ecs::CheeseGuyserBuiltOn { pump: pump_entity }
    ));
}

fn spawn_units_in_circle(
    buffer: &mut CommandBuffer,
    animations: &ModelAnimations,
    num: u32,
    center: Vec2,
    facing: f32,
    side: ecs::Side,
) {
    for i in 0..num {
        let rads = (i as f32 / num as f32 * 360.0).to_radians();
        ecs::Unit::MouseMarine.add_to_world(
            buffer,
            Some(animations),
            center + Vec2::new(rads.sin(), rads.cos()),
            ecs::Facing(facing.to_radians()),
            side,
            None,
        );
    }
}

// Single engineer has to build a base.
fn two(world: &mut World) {}
