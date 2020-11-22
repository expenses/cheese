use super::{ActionState, Building, BuildingCompleteness, Command, CommandQueue, Health};
use legion::{world::SubWorld, IntoQuery};

#[legion::system(for_each)]
#[read_component(Building)]
#[write_component(Health)]
#[write_component(BuildingCompleteness)]
pub fn build_buildings(command_queue: &mut CommandQueue, world: &mut SubWorld) {
    let mut pop_front = false;

    if let Some(Command::Build {
        target,
        state: ActionState::InRange,
    }) = command_queue.0.front()
    {
        let (building, mut health, mut completeness) =
            <(&Building, &mut Health, &mut BuildingCompleteness)>::query()
                .get_mut(world, *target)
                .expect("We've cancelled actions on dead entities");

        let max = building.stats().max_health;

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
