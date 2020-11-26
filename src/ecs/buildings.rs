use super::{
    nearest_point_within_building, ActionState, Building, BuildingCompleteness,
    CheeseGuyserBuiltOn, Command, CommandQueue, Cooldown, Facing, Health, Position,
    RecruitmentQueue, Side,
};
use crate::assets::ModelAnimations;
use crate::resources::{CheeseCoins, DeltaTime, PlayerSide};
use legion::{component, systems::CommandBuffer, world::SubWorld, Entity, EntityStore, IntoQuery};

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

#[legion::system(for_each)]
pub fn progress_recruitment_queue(
    building_position: &Position,
    building: &Building,
    recruitment_queue: &mut RecruitmentQueue,
    completeness: &BuildingCompleteness,
    side: &Side,
    #[resource] animations: &ModelAnimations,
    #[resource] delta_time: &DeltaTime,
    buffer: &mut CommandBuffer,
) {
    // Todo: a `Complete` tag would be nice.
    if completeness.0 != building.stats().max_health {
        return;
    }

    if let Some(unit) = recruitment_queue.queue.front().cloned() {
        let recruitment_time = unit.stats().recruitment_time;
        recruitment_queue.progress += delta_time.0;
        if recruitment_queue.progress > recruitment_time {
            recruitment_queue.progress -= recruitment_time;
            recruitment_queue.queue.pop_front();

            let start_point = nearest_point_within_building(
                recruitment_queue.waypoint,
                unit.stats().radius,
                building_position.0,
                building.stats().dimensions,
            );

            let command = Command::MoveTo {
                target: recruitment_queue.waypoint,
                attack_move: true,
                path: Vec::new(),
            };
            unit.add_to_world(
                buffer,
                Some(animations),
                start_point,
                Facing(0.0),
                *side,
                Some(command),
            );
        }
    } else {
        // If a unit just finished off the queue and there are no more units in the queue,
        // we don't want to keep the carry-over progress from the last unit around.
        recruitment_queue.progress = 0.0;
    }
}

#[legion::system(for_each)]
// I think we need this :^(
#[read_component(Position)]
pub fn free_up_cheese_guysers(
    entity: &Entity,
    built_on: &CheeseGuyserBuiltOn,
    buffer: &mut CommandBuffer,
    world: &SubWorld,
) {
    if world.entry_ref(built_on.pump).is_err() {
        buffer.remove_component::<CheeseGuyserBuiltOn>(*entity);
    }
}
