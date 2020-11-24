use super::{
    ActionState, Building, BuildingCompleteness, Command, CommandQueue, Cooldown, Facing, Health,
    Position, Side,
};
use crate::resources::{CheeseCoins, PlayerSide};
use legion::{component, world::SubWorld, Entity, IntoQuery};

#[legion::system(for_each)]
#[filter(component::<Position>())]
#[read_component(Position)]
#[read_component(Building)]
#[write_component(Health)]
#[write_component(BuildingCompleteness)]
pub fn build_buildings(
    entity: &Entity,
    command_queue: &mut CommandQueue,
    facing: &mut Facing,
    world: &mut SubWorld,
) {
    let mut pop_front = false;

    let position = <&Position>::query()
        .get(world, *entity)
        .expect("We've applied a filter to this system for Position")
        .0;

    if let Some(Command::Build {
        target,
        state: ActionState::InRange,
    }) = command_queue.0.front()
    {
        let (building_pos, building, mut health, mut completeness) =
            <(&Position, &Building, &mut Health, &mut BuildingCompleteness)>::query()
                .get_mut(world, *target)
                .expect("We've cancelled actions on dead entities");

        let max = building.stats().max_health;

        let vector = building_pos.0 - position;
        facing.0 = vector.y.atan2(vector.x);

        health.0 = (health.0 + 1).min(max);
        completeness.0 = (completeness.0 + 1).min(max);

        if health.0 == max {
            pop_front = true;
        }
    }

    if pop_front {
        command_queue.0.pop_front();
    }
}

#[legion::system(for_each)]
pub fn generate_cheese_coins(
    building: &Building,
    completeness: &BuildingCompleteness,
    side: &Side,
    cooldown: &mut Cooldown,
    #[resource] player_side: &PlayerSide,
    #[resource] cheese_coins: &mut CheeseCoins,
) {
    if cooldown.0 == 0.0
        && building == &Building::Pump
        && completeness.0 == building.stats().max_health
        && side == &player_side.0
    {
        cheese_coins.0 += 2;
        cooldown.0 = 0.5;
    }
}
