use crate::renderer::{Instance, InstanceBuffers, Vertex};
use crate::resources::{
    Camera, CameraControls, MouseState, PlayerSide, RtsControls, ScreenDimensions,
};
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use std::collections::VecDeque;
use ultraviolet::{Mat4, Vec2, Vec3};

mod combat;
mod controls;
mod movement;
mod rendering;
pub use combat::*;
pub use controls::*;
pub use movement::*;
pub use rendering::*;

#[derive(Debug)]
pub struct Position(pub Vec2);
pub struct Facing(pub f32);
#[derive(PartialEq)]
pub enum Side {
    Green,
    Purple,
}
pub struct Selected;
pub struct Selectable;

#[derive(Clone, Copy)]
pub enum Command {
    MoveTo(Vec2),
    Attack(Entity),
    AttackMove(Vec2),
}

#[derive(Default)]
pub struct CommandQueue(VecDeque<Command>);

pub struct Health(pub u16);

#[derive(Debug)]
pub struct Bullet {
    target: Entity,
}

#[derive(Debug)]
pub struct MoveTo(pub Vec2);

pub struct FiringCooldown(pub u8);

const FIRING_RANGE: f32 = 10.0;
const MOVE_SPEED: f32 = 0.1;
const SELECTION_RADIUS: f32 = 2.0;

fn sort_points(a: Vec2, b: Vec2) -> (Vec2, Vec2) {
    (
        Vec2::new(a.x.min(b.x), a.y.min(b.y)),
        Vec2::new(a.x.max(b.x), a.y.max(b.y)),
    )
}
