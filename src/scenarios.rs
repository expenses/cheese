use crate::assets::ModelAnimations;
use crate::ecs;
use crate::pathfinding::Map;
use crate::resources::{
    AiBuildOrderItem, AiBuildOrders, Camera, CheeseCoins, LoseCondition, Objectives, WinCondition,
};
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
    ai_build_orders: &mut AiBuildOrders,
) {
    let mut command_buffer = legion::systems::CommandBuffer::new(&world);

    let unit_spawn_point = Vec2::new(-36.0, 0.0);

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        10,
        ecs::Unit::MouseMarine,
        unit_spawn_point,
        2.0,
        0.0,
        ecs::Side::Green,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        5,
        ecs::Unit::MouseMarine,
        Vec2::new(40.0, 6.7),
        1.0,
        180.0,
        ecs::Side::Purple,
    );

    command_buffer.flush(world);

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
    *ai_build_orders = AiBuildOrders::default();
}

fn spawn_guyser(world: &mut World, position: Vec2) -> Entity {
    world.push((
        ecs::Position(position),
        ecs::CheeseGuyser,
        ecs::Cooldown(0.0),
    ))
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
    unit: ecs::Unit,
    center: Vec2,
    radius: f32,
    facing: f32,
    side: ecs::Side,
) {
    for i in 0..num {
        let rads = (i as f32 / num as f32 * 360.0).to_radians();
        unit.add_to_world(
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
    ai_build_orders: &mut AiBuildOrders,
) {
    let engineer_pos = Vec2::new(-52.69, -53.42);

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
        ecs::Unit::MouseMarine,
        enemy_pos + Vec2::new(-3.0, 0.0),
        1.0,
        direction.y.atan2(direction.x).to_degrees(),
        ecs::Side::Purple,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        5,
        ecs::Unit::MouseMarine,
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
    *ai_build_orders = AiBuildOrders::default();
}

pub fn three(
    mut world: &mut World,
    animations: &ModelAnimations,
    map: &mut Map,
    rng: &mut rand::rngs::SmallRng,
    objectives: &mut Objectives,
    camera: &mut Camera,
    cheese_coins: &mut CheeseCoins,
    ai_build_orders: &mut AiBuildOrders,
) {
    let start = Vec2::new(-57.57, -59.81);

    let mut command_buffer = legion::systems::CommandBuffer::new(&world);

    ecs::Unit::Engineer.add_to_world(
        &mut command_buffer,
        Some(animations),
        start,
        ecs::Facing(0.0),
        ecs::Side::Green,
        None,
    );

    ecs::Unit::Engineer.add_to_world(
        &mut command_buffer,
        Some(animations),
        -start,
        ecs::Facing(0.0),
        ecs::Side::Purple,
        None,
    );

    command_buffer.flush(world);

    let base_guysers = [
        Vec2::new(-72.23, -78.57),
        Vec2::new(-74.96, -63.91),
        Vec2::new(-46.65, -78.57),
        Vec2::new(-56.55, -33.89),
        Vec2::new(-25.85, -60.5),
    ];

    let mut enemy_guyser_entities = Vec::new();

    let center_guyser = spawn_guyser(&mut world, Vec2::zero());
    for guyser in &base_guysers {
        spawn_guyser(&mut world, *guyser);
        enemy_guyser_entities.push(spawn_guyser(&mut world, -*guyser));
    }

    *objectives = Objectives {
        win_conditions: vec![WinCondition::DestroyAll],
        lose_conditions: vec![LoseCondition::LetAllUnitsDie],
    };

    *camera = Camera {
        looking_at: start,
        distance: 30.0,
    };

    *cheese_coins = CheeseCoins(100);

    *ai_build_orders = AiBuildOrders(vec![
        (
            5.3602057,
            AiBuildOrderItem::BuildPump(enemy_guyser_entities[0]),
        ),
        (
            12.738736,
            AiBuildOrderItem::BuildPump(enemy_guyser_entities[1]),
        ),
        (
            23.946945,
            AiBuildOrderItem::BuildPump(enemy_guyser_entities[2]),
        ),
        (
            31.270610,
            AiBuildOrderItem::BuildPump(enemy_guyser_entities[4]),
        ),
        (
            39.601414,
            AiBuildOrderItem::BuildPump(enemy_guyser_entities[3]),
        ),
        (
            49.388996,
            AiBuildOrderItem::BuildArmoury(Vec2::new(35.87867, 38.60826)),
        ),
        (
            60.0,
            AiBuildOrderItem::SetWaypoint(Vec2::new(26.272, 30.529)),
        ),
        (60.0, AiBuildOrderItem::RecruitMarine(100)),
        (
            103.63205,
            AiBuildOrderItem::AttackMove(Vec2::new(-8.6, -4.9)),
        ),
        (111.126175, AiBuildOrderItem::BuildPump(center_guyser)),
        (
            114.38,
            AiBuildOrderItem::AttackMove(Vec2::new(-16.08, -7.71)),
        ),
        (
            125.53555,
            AiBuildOrderItem::BuildArmoury(Vec2::new(8.027, -2.995)),
        ),
        (
            134.9838,
            AiBuildOrderItem::AttackMove(Vec2::new(-64.785, -35.520)),
        ),
        (
            142.98,
            AiBuildOrderItem::SetWaypoint(Vec2::new(-4.4166, -13.563)),
        ),
        (
            145.0,
            AiBuildOrderItem::AttackMove(Vec2::new(-28.534466, -63.937397)),
        ),
        (
            155.0,
            AiBuildOrderItem::AttackMove(Vec2::new(-57.537994, -69.26898)),
        ),
        (
            165.0,
            AiBuildOrderItem::AttackMove(Vec2::new(-78.44591, -80.18292)),
        ),
        (
            175.0,
            AiBuildOrderItem::AttackMove(Vec2::new(-46.487736, -79.067795)),
        ),
    ]);
}

// Here's a list of moves that I made when I played as the enemy side:

/*
5.3602057: Pump RayCastLocation { pos: Vec2 { x: 72.23, y: 78.57 }, snapped_to_guyser: Some(Entity(63)) }
12.738736: Pump RayCastLocation { pos: Vec2 { x: 74.96, y: 63.91 }, snapped_to_guyser: Some(Entity(95)) }
23.44634: MoveTo { target: Vec2 { x: 59.986694, y: 71.15368 }, attack_move: false, path: [] }
23.946945: Pump RayCastLocation { pos: Vec2 { x: 46.65, y: 78.57 }, snapped_to_guyser: Some(Entity(127)) }
31.27061: Pump RayCastLocation { pos: Vec2 { x: 25.85, y: 60.5 }, snapped_to_guyser: Some(Entity(191)) }
39.601414: Pump RayCastLocation { pos: Vec2 { x: 56.55, y: 33.89 }, snapped_to_guyser: Some(Entity(159)) }
49.388996: Armoury RayCastLocation { pos: Vec2 { x: 35.87867, y: 38.60826 }, snapped_to_guyser: None }
59.695637: setting waypoint Vec2 { x: 26.272722, y: 30.529737 }
61.196625: building MouseMarine
61.38647: building MouseMarine
62.87598: building MouseMarine
66.2489: MoveTo { target: Vec2 { x: 44.887505, y: 28.902779 }, attack_move: false, path: [] }
69.42682: building MouseMarine
73.06995: building MouseMarine
79.582565: building MouseMarine
83.561134: building MouseMarine
88.373856: building MouseMarine
93.2919: building MouseMarine
103.63205: MoveTo { target: Vec2 { x: -8.602108, y: -4.904587 }, attack_move: true, path: [] }
111.126175: Pump RayCastLocation { pos: Vec2 { x: 0.0, y: 0.0 }, snapped_to_guyser: Some(Entity(31)) }
114.38201: MoveTo { target: Vec2 { x: -16.081762, y: -7.714676 }, attack_move: true, path: [] }
125.53555: Armoury RayCastLocation { pos: Vec2 { x: 8.027401, y: -2.9950018 }, snapped_to_guyser: None }
134.9838: MoveTo { target: Vec2 { x: -64.78539, y: -35.520306 }, attack_move: true, path: [] }
138.8399: setting waypoint Vec2 { x: -6.710074, y: -15.923779 }
142.97922: setting waypoint Vec2 { x: -4.4166384, y: -13.563568 }
143.42065: building MouseMarine
143.59573: building MouseMarine
143.77641: building MouseMarine
143.93388: building MouseMarine
146.01065: MoveTo { target: Vec2 { x: -28.534466, y: -63.937397 }, attack_move: true, path: [] }
148.50993: MoveTo { target: Vec2 { x: -57.537994, y: -69.26898 }, attack_move: true, path: [] }
149.3153: MoveTo { target: Vec2 { x: -78.44591, y: -80.18292 }, attack_move: true, path: [] }
150.26266: MoveTo { target: Vec2 { x: -46.487736, y: -79.067795 }, attack_move: true, path: [] }
*/

pub fn sandbox(
    world: &mut World,
    animations: &ModelAnimations,
    map: &mut Map,
    rng: &mut rand::rngs::SmallRng,
    objectives: &mut Objectives,
    camera: &mut Camera,
    cheese_coins: &mut CheeseCoins,
    ai_build_orders: &mut AiBuildOrders,
) {
    let mut command_buffer = legion::systems::CommandBuffer::new(&world);

    ecs::Unit::Engineer.add_to_world(
        &mut command_buffer,
        Some(animations),
        Vec2::new(0.0, -90.0),
        ecs::Facing(0.0),
        ecs::Side::Green,
        None,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        25,
        ecs::Unit::MouseMarine,
        Vec2::new(0.0, 90.0),
        1.0,
        0.0,
        ecs::Side::Purple,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        25,
        ecs::Unit::MouseMarine,
        Vec2::new(0.0, 90.0),
        2.0,
        0.0,
        ecs::Side::Purple,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        25,
        ecs::Unit::MouseMarine,
        Vec2::new(0.0, 90.0),
        3.0,
        0.0,
        ecs::Side::Purple,
    );

    spawn_units_in_circle(
        &mut command_buffer,
        animations,
        25,
        ecs::Unit::MouseMarine,
        Vec2::new(0.0, 90.0),
        4.0,
        0.0,
        ecs::Side::Purple,
    );

    command_buffer.flush(world);

    *objectives = Objectives {
        win_conditions: vec![],
        lose_conditions: vec![],
    };
    *camera = Camera {
        looking_at: Vec2::new(0.0, -90.0),
        distance: 50.0,
    };
    *cheese_coins = CheeseCoins(10_000_000);
    *ai_build_orders = AiBuildOrders::default();
}
