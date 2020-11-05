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

pub struct FiringRange(pub f32);
pub struct MoveSpeed(pub f32);
pub struct Radius(pub f32);

pub struct DamagedThisTick(pub Entity);

#[derive(Debug)]
pub struct Bullet {
    source: Entity,
    target: Entity,
}

#[derive(Debug)]
pub struct MoveTo(pub Vec2);

pub struct FiringCooldown(pub u8);

fn sort_points(a: Vec2, b: Vec2) -> (Vec2, Vec2) {
    (
        Vec2::new(a.x.min(b.x), a.y.min(b.y)),
        Vec2::new(a.x.max(b.x), a.y.max(b.y)),
    )
}

struct SelectBox {
    top_left: Vec2,
    top_right: Vec2,
    bottom_left: Vec2,
    bottom_right: Vec2,
}

impl SelectBox {
    fn new(
        camera: &Camera,
        screen_dimensions: &ScreenDimensions,
        start: Vec2,
        current: Vec2,
    ) -> Self {
        let (top_left, bottom_right) = sort_points(start, current);
        let (left, right, top, bottom) = (top_left.x, bottom_right.x, top_left.y, bottom_right.y);

        Self {
            top_left: camera.cast_ray(Vec2::new(left, top), screen_dimensions),
            top_right: camera.cast_ray(Vec2::new(right, top), screen_dimensions),
            bottom_left: camera.cast_ray(Vec2::new(left, bottom), screen_dimensions),
            bottom_right: camera.cast_ray(Vec2::new(right, bottom), screen_dimensions),
        }
    }

    fn contains(&self, point: Vec2) -> bool {
        let point = vec2_to_ncollide_point(point);
        let top_left_point = vec2_to_ncollide_point(self.top_left);
        let top_right_point = vec2_to_ncollide_point(self.top_right);
        let bottom_left_point = vec2_to_ncollide_point(self.bottom_left);
        let bottom_right_point = vec2_to_ncollide_point(self.bottom_right);

        ncollide3d::utils::is_point_in_triangle(
            &point,
            &top_left_point,
            &top_right_point,
            &bottom_left_point,
        ) || ncollide3d::utils::is_point_in_triangle(
            &point,
            &top_right_point,
            &bottom_left_point,
            &bottom_right_point,
        )
    }
}

fn vec2_to_ncollide_point(point: Vec2) -> ncollide3d::math::Point<f32> {
    ncollide3d::math::Point::new(point.x, 0.0, point.y)
}
