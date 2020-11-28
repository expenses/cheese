use super::{
    CheeseDropletPosition, CheeseDropletVelocity, CheeseGuyser, CheeseGuyserBuiltOn, Cooldown,
    Explosion, Position,
};
use crate::renderer::{ModelBuffers, ModelInstance};
use crate::resources::{DeltaTime, Gravity};
use legion::{component, systems::CommandBuffer, Entity};
use rand::Rng;
use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};

#[legion::system(for_each)]
pub fn apply_gravity(
    velocity: &mut CheeseDropletVelocity,
    #[resource] gravity: &Gravity,
    #[resource] delta_time: &DeltaTime,
) {
    velocity.0.y -= gravity.0 * delta_time.0;
}

#[legion::system(for_each)]
pub fn move_cheese_droplets(
    entity: &Entity,
    position: &mut CheeseDropletPosition,
    velocity: &CheeseDropletVelocity,
    buffer: &mut CommandBuffer,
    #[resource] delta_time: &DeltaTime,
) {
    position.0 += velocity.0 * delta_time.0;
    if position.0.y < -1.0 {
        buffer.remove(*entity);
    }
}

#[legion::system(for_each)]
#[filter(component::<CheeseGuyser>() & !component::<CheeseGuyserBuiltOn>())]
pub fn spawn_cheese_droplets(
    position: &Position,
    #[resource] rng: &mut rand::rngs::SmallRng,
    buffer: &mut CommandBuffer,
    cooldown: &mut Cooldown,
) {
    if cooldown.0 != 0.0 {
        return;
    }

    for _ in 0..3 {
        let rotation = rng.gen_range(0.0, std::f32::consts::TAU);
        let velocity = Vec3::new(rotation.cos() * 0.75, 10.0, rotation.sin() * 0.75);
        buffer.push((
            CheeseDropletPosition(Vec3::new(position.0.x, 0.0, position.0.y)),
            CheeseDropletVelocity(velocity),
        ));
        cooldown.0 = 1.0 / 60.0;
    }
}

#[legion::system(for_each)]
pub fn render_cheese_droplets(
    position: &CheeseDropletPosition,
    velocity: &CheeseDropletVelocity,
    #[resource] model_buffers: &mut ModelBuffers,
) {
    let translation = Mat4::from_translation(position.0);
    let rotation = Rotor3::from_rotation_between(-Vec3::unit_y(), velocity.0.normalized())
        .into_matrix()
        .into_homogeneous();
    model_buffers.cheese_droplets.push(ModelInstance {
        transform: translation * rotation,
        flat_colour: Vec4::one(),
    });
}

#[legion::system(for_each)]
pub fn render_explosions(explosion: &Explosion, #[resource] model_buffers: &mut ModelBuffers) {
    model_buffers.explosions.push(ModelInstance {
        transform: explosion.translation_rotation * Mat4::from_scale(explosion.size()),
        flat_colour: Vec4::new(1.0, 1.0, 1.0, 1.0 / 3.0),
    });
}

#[legion::system(for_each)]
pub fn expand_explosions(
    entity: &Entity,
    explosion: &mut Explosion,
    #[resource] delta_time: &DeltaTime,
    buffer: &mut CommandBuffer,
) {
    explosion.progress += 3.0 * delta_time.0 / explosion.duration();

    if explosion.progress > 1.0 {
        buffer.remove(*entity);
    }
}
