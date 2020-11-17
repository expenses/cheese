use crate::pathfinding::{Map, MapHandle};
use crate::resources::{
    Camera, CameraControls, MouseState, PlayerSide, RtsControls, ScreenDimensions,
};
use crate::Assets;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use std::collections::VecDeque;
use ultraviolet::{Mat4, Vec2, Vec3};

mod animation;
mod combat;
mod controls;
mod debugging;
mod movement;
mod rendering;
pub use animation::*;
pub use combat::*;
pub use controls::*;
pub use debugging::*;
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

#[derive(Default)]
pub struct MovementDebugging {
    triangles: Vec<(Vec2, Vec2)>,
    funnel_points: Vec<(Vec2, Vec2)>,
    path_start: Vec2,
    path_end: Vec2,
}

#[derive(Clone)]
pub enum Command {
    MoveTo {
        target: Vec2,
        // Should we stop and attack things on the way?
        attack_move: bool,
        path: Vec<Vec2>,
    },
    Attack {
        target: Entity,
        // Was the unit explicitly commanded to attack, or was this caused by attack moving or agro?
        // todo: attack moves need to give up when an enemy goes out of range.
        explicit: bool,
        // Is the unit out of range for the first time? If so, go within range no matter what.
        // If it's not an explicit attack and we're not out of range for the first time, then it's
        // better to just switch targets than to chase. We set this to true initially and just 'and'
        // it with whether the unit is out of range.
        first_out_of_range: bool,
        state: AttackState,
    },
}

impl Command {
    fn path(&self) -> Option<&Vec<Vec2>> {
        if let &Command::MoveTo { ref path, .. }
        | &Command::Attack {
            state: AttackState::OutOfRange { ref path },
            ..
        } = self
        {
            Some(path)
        } else {
            None
        }
    }

    fn path_mut(&mut self) -> Option<&mut Vec<Vec2>> {
        if let &mut Command::MoveTo { ref mut path, .. }
        | &mut Command::Attack {
            state: AttackState::OutOfRange { ref mut path },
            ..
        } = self
        {
            Some(path)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub enum AttackState {
    OutOfRange { path: Vec<Vec2> },
    InRange,
}

impl AttackState {
    fn is_out_of_range(&self) -> bool {
        matches!(self, Self::OutOfRange { .. })
    }
}

#[derive(Default)]
pub struct CommandQueue(VecDeque<Command>);

pub struct Health(pub u16);

pub struct FiringRange(pub f32);
pub struct MoveSpeed(pub f32);
pub struct Radius(pub f32);

pub struct DamagedThisTick(pub Entity);

pub struct AnimationState {
    pub animation: usize,
    pub time: f32,
    pub total_time: f32,
}

#[derive(Debug)]
pub struct Bullet {
    source: Entity,
    target: Entity,
    target_position: Vec2,
}

#[derive(Debug)]
pub struct MoveTo(pub Vec2);

pub struct FiringCooldown(pub u8);

pub struct CommandGroup {
    entities: Vec<Entity>,
}

pub struct Building {
    pub handle: MapHandle,
    position: Vec2,
}

impl Building {
    pub fn new(center: Vec2, dimensions: Vec2, map: &mut Map) -> Option<Self> {
        Some(Self {
            handle: map.insert(center, dimensions)?,
            position: center,
        })
    }
}

#[derive(Copy, Clone)]
pub enum Unit {
    MouseMarine,
    Hulk,
}

pub struct UnitStats {
    pub max_health: u16,
    pub move_speed: f32,
    pub radius: f32,
    pub firing_range: f32,
    pub health_bar_height: f32,
}

enum MouseAnimation {
    Idle = 0,
    Walking = 1,
}

impl Unit {
    fn stats(self) -> UnitStats {
        match self {
            Self::MouseMarine => UnitStats {
                max_health: 50,
                firing_range: 10.0,
                move_speed: 6.0,
                radius: 1.0,
                health_bar_height: 3.0,
            },
            Self::Hulk => UnitStats {
                max_health: 500,
                firing_range: 5.0,
                move_speed: 6.0,
                radius: 1.5,
                health_bar_height: 3.0,
            },
        }
    }

    pub fn add_to_world(
        self,
        world: &mut World,
        // This is only `None` when being run in a test
        assets: Option<&Assets>,
        position: Vec2,
        facing: Facing,
        side: Side,
    ) -> Entity {
        let UnitStats {
            max_health,
            move_speed,
            radius,
            firing_range,
            health_bar_height: _,
        } = self.stats();

        let entity = world.push((
            Position(position),
            facing,
            side,
            self,
            CommandQueue::default(),
            Avoids,
            Avoidable,
            Selectable,
            Health(max_health),
            FiringCooldown(0),
            FiringRange(firing_range),
            MoveSpeed(move_speed),
            Radius(radius),
            // Uncomment to debug movement.
            // MovementDebugging::default(),
        ));

        if let Some(assets) = assets {
            let mut entry = world.entry(entity).unwrap();

            entry.add_component(assets.mouse_model.skin.clone());
            entry.add_component(AnimationState {
                animation: MouseAnimation::Idle as usize,
                time: 0.0,
                total_time: assets.mouse_model.animations[MouseAnimation::Idle as usize].total_time,
            });
        }

        entity
    }
}

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
