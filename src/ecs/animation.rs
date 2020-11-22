use super::{
    AnimationState, Building, BuildingCompleteness, Command, CommandQueue, MouseAnimation,
};
use crate::animation::Skin;
use crate::assets::ModelAnimations;
use crate::resources::DeltaTime;

#[legion::system(for_each)]
pub fn progress_animations(
    skin: &mut Skin,
    animation_state: &mut AnimationState,
    commands: &CommandQueue,
    #[resource] animations: &ModelAnimations,
    #[resource] delta_time: &DeltaTime,
) {
    let animation = match commands.0.front() {
        Some(&Command::MoveTo { .. }) => MouseAnimation::Walking,
        Some(&Command::Attack { ref state, .. }) => {
            if state.is_out_of_range() {
                MouseAnimation::Walking
            } else {
                MouseAnimation::Shoot
            }
        }
        Some(&Command::Build { ref state, .. }) => {
            if state.is_out_of_range() {
                MouseAnimation::Walking
            } else {
                MouseAnimation::Build
            }
        }
        None => MouseAnimation::Idle,
    } as usize;

    if animation != animation_state.animation {
        animation_state.animation = animation;
        animation_state.time = 0.0;
        animation_state.total_time = animations.mouse.animations[animation].total_time;
    } else {
        animation_state.time += delta_time.0;
        animation_state.time %= animation_state.total_time;
    }

    animations.mouse.animations[animation_state.animation].animate(skin, animation_state.time);
}

#[legion::system(for_each)]
pub fn progress_building_animations(
    building: &Building,
    completeness: &BuildingCompleteness,
    skin: &mut Skin,
    animation_state: &mut AnimationState,
    #[resource] animations: &ModelAnimations,
    #[resource] delta_time: &DeltaTime,
) {
    if completeness.0 != building.stats().max_health {
        return;
    }

    animation_state.time += delta_time.0;
    animation_state.time %= animation_state.total_time;

    match building {
        Building::Pump => {
            animations.pump.animations[animation_state.animation]
                .animate(skin, animation_state.time);
        }
        Building::Armoury => {}
    }
}
