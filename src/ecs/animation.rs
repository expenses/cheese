use super::{AnimationState, Command, CommandQueue};
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
        Some(&Command::MoveTo(_)) | Some(&Command::AttackMove(_)) => 1,
        _ => 0,
    };

    if animation != animation_state.animation {
        animation_state.animation = animation;
        animation_state.time = 0.0;
        animation_state.total_time = assets.mouse_model.animations[animation].total_time;
    } else {
        animation_state.time += delta_time.0;
        animation_state.time = animation_state.time % animation_state.total_time;
    }

    assets.mouse_model.animations[animation_state.animation].animate(skin, animation_state.time);
}