use super::{AnimationState, Building, Command, CommandQueue, MouseAnimation};
use crate::animation::Skin;
use crate::assets::Assets;
use crate::resources::DeltaTime;

#[legion::system(for_each)]
pub fn progress_animations(
    skin: &mut Skin,
    animation_state: &mut AnimationState,
    commands: &CommandQueue,
    #[resource] assets: &Assets,
    #[resource] delta_time: &DeltaTime,
) {
    let animation = match commands.0.front() {
        Some(&Command::MoveTo { .. }) => MouseAnimation::Walking,
        Some(&Command::Attack { ref state, .. }) => {
            if state.is_out_of_range() {
                MouseAnimation::Walking
            } else {
                MouseAnimation::Idle
            }
        }
        None => MouseAnimation::Idle,
    } as usize;

    if animation != animation_state.animation {
        animation_state.animation = animation;
        animation_state.time = 0.0;
        animation_state.total_time = assets.mouse_model.animations[animation].total_time;
    } else {
        animation_state.time += delta_time.0;
        animation_state.time %= animation_state.total_time;
    }

    assets.mouse_model.animations[animation_state.animation].animate(skin, animation_state.time);
}

#[legion::system(for_each)]
pub fn progress_building_animations(
    building: &Building,
    skin: &mut Skin,
    animation_state: &mut AnimationState,
    #[resource] assets: &Assets,
    #[resource] delta_time: &DeltaTime,
) {
    animation_state.time += delta_time.0;
    animation_state.time %= animation_state.total_time;

    match building {
        Building::Pump => {
            assets.pump_model.animations[animation_state.animation]
                .animate(skin, animation_state.time);
        }
        Building::Armoury => {}
    }
}
