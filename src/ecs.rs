use crate::renderer::{Instance, InstanceBuffers, Vertex};
use crate::resources::{
    Camera, CameraControls, MouseState, PlayerSide, RtsControls, ScreenDimensions,
};
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use std::collections::VecDeque;
use ultraviolet::{Mat4, Vec2, Vec3};

mod controls;
mod movement;
mod rendering;
pub use controls::*;
pub use movement::*;
pub use rendering::*;

pub struct Position(pub Vec2);
pub struct Facing(pub f32);
#[derive(PartialEq)]
pub enum Side {
    Green,
    Purple,
}
pub struct Selected;
pub struct Selectable;

#[derive(Clone)]
pub enum Command {
    MoveTo(Vec2),
    Attack(Entity),
}

#[derive(Default)]
pub struct CommandQueue(VecDeque<Command>);

pub struct Health(u16);

pub struct Bullet;

const FIRING_RANGE: f32 = 5.0;
const MOVE_SPEED: f32 = 0.1;

#[legion::system(for_each)]
pub fn stop_attacks_on_dead_entities(commands: &mut CommandQueue, world: &SubWorld) {
    while commands
        .0
        .front()
        .map(|command| {
            if let Command::Attack(entity) = command {
                world.entry_ref(*entity).is_err()
            } else {
                false
            }
        })
        .unwrap_or(false)
    {
        commands.0.pop_front();
    }
}

fn sort_points(a: Vec2, b: Vec2) -> (Vec2, Vec2) {
    (
        Vec2::new(a.x.min(b.x), a.y.min(b.y)),
        Vec2::new(a.x.max(b.x), a.y.max(b.y)),
    )
}
