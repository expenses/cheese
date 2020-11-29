use super::*;
use crate::assets::ModelAnimations;
use crate::pathfinding::Map;
use crate::resources::{AiBuildOrderItem, AiBuildOrders, PlayerSide, TotalTime};

// I sorta ran out of time/mental energy to implement proper AI, so I just have it follow
// pre-recorded instructions.

#[legion::system]
#[read_component(Position)]
#[read_component(Side)]
#[read_component(Unit)]
#[write_component(CommandQueue)]
#[write_component(RecruitmentQueue)]
pub fn follow_ai_build_orders(
    #[resource] build_orders: &mut AiBuildOrders,
    #[resource] total_time: &TotalTime,
    #[resource] map: &mut Map,
    #[resource] animations: &ModelAnimations,
    #[resource] player_side: &PlayerSide,
    world: &mut SubWorld,
    commands: &mut CommandBuffer,
) {
    let mut remove_first = false;
    if let Some((time, item)) = build_orders.0.first() {
        if *time <= total_time.0 {
            log::debug!(target: "ai", "Following build order: {:?}", item);

            match item {
                AiBuildOrderItem::BuildPump(guyser_entity) => {
                    let position = <&Position>::query().get(world, *guyser_entity).unwrap();

                    let pump_entity = Building::Pump
                        .add_to_world_to_construct(
                            commands,
                            position.0,
                            player_side.0.flip(),
                            animations,
                            map,
                        )
                        .unwrap();

                    commands
                        .add_component(*guyser_entity, CheeseGuyserBuiltOn { pump: pump_entity });

                    <(&mut CommandQueue, &Side)>::query()
                        .filter(component::<CanBuild>())
                        .iter_mut(world)
                        .filter(|(_, side)| **side != player_side.0)
                        .for_each(|(commands, _)| {
                            commands.0.clear();
                            commands.0.push_back(Command::new_build(pump_entity));
                        });
                }
                AiBuildOrderItem::BuildArmoury(position) => {
                    let armoury_entity = Building::Armoury
                        .add_to_world_to_construct(
                            commands,
                            *position,
                            player_side.0.flip(),
                            animations,
                            map,
                        )
                        .unwrap();

                    <(&mut CommandQueue, &Side)>::query()
                        .filter(component::<CanBuild>())
                        .iter_mut(world)
                        .filter(|(_, side)| **side != player_side.0)
                        .for_each(|(commands, _)| {
                            commands.0.clear();
                            commands.0.push_back(Command::new_build(armoury_entity));
                        });
                }
                AiBuildOrderItem::RecruitMarine(times) => {
                    for _ in 0..*times {
                        let shortest_queue = <(&mut RecruitmentQueue, &Side)>::query()
                            .iter_mut(world)
                            .filter(|(_, side)| **side != player_side.0)
                            .min_by_key(|(queue, _)| queue.length());

                        if let Some((queue, _)) = shortest_queue {
                            queue.queue.push_back(Unit::MouseMarine);
                        }
                    }
                }
                AiBuildOrderItem::SetWaypoint(position) => {
                    <(&mut RecruitmentQueue, &Side)>::query()
                        .iter_mut(world)
                        .filter(|(_, side)| **side != player_side.0)
                        .for_each(|(queue, _)| {
                            queue.waypoint = *position;
                        })
                }
                AiBuildOrderItem::AttackMove(position) => {
                    <(&mut CommandQueue, &Side, &Unit)>::query()
                        .iter_mut(world)
                        .filter(|(_, side, unit)| {
                            **side != player_side.0 && **unit == Unit::MouseMarine
                        })
                        .for_each(|(commands, ..)| {
                            commands.0.clear();
                            commands.0.push_back(Command::MoveTo {
                                target: *position,
                                attack_move: true,
                                path: Vec::new(),
                            });
                        })
                }
            }

            remove_first = true;
        }
    }

    if remove_first {
        build_orders.0.remove(0);
    }
}
