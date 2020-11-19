use super::{EffectPosition, EffectVelocity, EffectRotation, CheeseGuyser, Position, ParticleType, Bounce};
use crate::renderer::{ModelBuffers, ModelInstance};
use crate::resources::{DeltaTime, Gravity};
use legion::{component, systems::CommandBuffer, Entity};
use rand::Rng;
use ultraviolet::{Mat4, Vec3, Vec4};

#[legion::system(for_each)]
pub fn apply_effect_gravity(
    velocity: &mut EffectVelocity,
    #[resource] gravity: &Gravity,
    #[resource] delta_time: &DeltaTime,
) {
    velocity.0.y -= gravity.0 * delta_time.0;
}

#[legion::system(for_each)]
pub fn apply_effect_velocity(
    entity: &Entity,
    position: &mut EffectPosition,
    velocity: &mut EffectVelocity,
    buffer: &mut CommandBuffer,
    bounce: Option<&Bounce>,
    #[resource] delta_time: &DeltaTime,
) {
    position.0 += velocity.0 * delta_time.0;

    if position.0.y < 0.0 && bounce.is_some() {
        position.0.y *= -1.0;
        velocity.0.y *= -0.5;
        buffer.remove_component::<Bounce>(*entity);
    } else if position.0.y < -1.0 {
        buffer.remove(*entity);
    }
}

#[legion::system(for_each)]
#[filter(component::<CheeseGuyser>())]
pub fn spawn_cheese_droplets(
    position: &Position,
    #[resource] rng: &mut rand::rngs::SmallRng,
    buffer: &mut CommandBuffer,
) {
    for _ in 0..3 {
        let rotation = rng.gen_range(0.0, std::f32::consts::TAU);
        let velocity = Vec3::new(rotation.cos() * 0.75, 12.5, rotation.sin() * 0.75);
        buffer.push((
            EffectPosition(Vec3::new(position.0.x, 0.0, position.0.y)),
            EffectVelocity(velocity),
            ParticleType::CheeseDroplet,
        ));
    }
}

#[legion::system(for_each)]
pub fn render_effects(
    position: &EffectPosition,
    velocity: &EffectVelocity,
    rotation: Option<&EffectRotation>,
    particle_type: &ParticleType,
    #[resource] model_buffers: &mut ModelBuffers,
) {
    let translation = Mat4::from_translation(position.0);
    let rotation = rotation.map(|rot| rot.0).unwrap_or_else(|| {
        ultraviolet::Rotor3::from_rotation_between(
            Vec3::new(0.0, -1.0, 0.0),
            velocity.0.normalized(),
        )
        .into_matrix()
        .into_homogeneous()
    });

    let buffer = match particle_type {
        ParticleType::CheeseDroplet => &mut model_buffers.cheese_droplets,
        ParticleType::Giblet => &mut model_buffers.giblets,
    };

    buffer.push(ModelInstance {
        transform: translation * rotation,
        flat_colour: Vec4::one(),
    });
}
