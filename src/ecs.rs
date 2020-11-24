use crate::assets::ModelAnimations;
use crate::pathfinding::{Map, MapHandle};
use crate::resources::{
    Camera, CameraControls, MouseState, PlayerSide, RtsControls, ScreenDimensions,
};
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use std::collections::VecDeque;
use ultraviolet::{Mat4, Vec2, Vec3};

mod animation;
mod buildings;
mod combat;
mod controls;
mod debugging;
mod effects;
mod movement;
mod rendering;

use crate::resources::DebugControls;
use animation::{progress_animations_system, progress_building_animations_system};
use buildings::{build_buildings_system, generate_cheese_coins_system};
use combat::{
    add_attack_commands_system, apply_bullets_system, firing_system, handle_damaged_system,
    reduce_cooldowns_system, stop_actions_on_dead_entities_system,
};
use controls::{
    cast_ray_system, control_camera_system, handle_control_groups_system,
    handle_drag_selection_system, handle_left_click_system, handle_right_click_system,
    handle_stop_command_system, remove_dead_entities_from_control_groups_system,
};
use debugging::{
    debug_select_box_system, debug_specific_path_system, render_building_grid_system,
    render_debug_unit_pathfinding_system, render_firing_ranges_system,
    render_pathfinding_map_system, render_unit_paths_system, set_debug_pathfinding_start_system,
    spawn_debug_building_system,
};
use effects::{
    apply_gravity_system, move_cheese_droplets_system, render_cheese_droplets_system,
    spawn_cheese_droplets_system,
};
use movement::{
    apply_steering_system, avoidance_system, move_bullets_system, move_units_system,
    reset_map_updated_system, set_movement_paths_system, Avoidable, Avoids,
};
use rendering::{
    render_building_plan_system, render_buildings_system, render_bullets_system,
    render_command_paths_system, render_drag_box_system, render_health_bars_system,
    render_selections_system, render_ui_system, render_under_select_box_system,
    render_unit_under_cursor_system, render_units_system,
};

#[legion::system]
pub fn cleanup_controls(
    #[resource] mouse_state: &mut MouseState,
    #[resource] rts_controls: &mut RtsControls,
    #[resource] debug_controls: &mut DebugControls,
) {
    let position = mouse_state.position;
    mouse_state.left_state.update(position);
    mouse_state.right_state.update(position);

    rts_controls.stop_pressed = false;

    for i in 0..10 {
        rts_controls.control_group_key_pressed[i] = false;
    }

    debug_controls.set_pathfinding_start_pressed = false;
}

pub fn add_gameplay_systems(builder: &mut legion::systems::Builder) {
    builder
        .add_system(generate_cheese_coins_system())
        .add_system(reset_map_updated_system())
        .add_system(cast_ray_system())
        .add_system(remove_dead_entities_from_control_groups_system())
        .add_system(stop_actions_on_dead_entities_system())
        .add_system(control_camera_system())
        .add_system(handle_left_click_system())
        .add_system(handle_right_click_system())
        .add_system(handle_stop_command_system())
        .add_system(handle_drag_selection_system())
        .add_system(handle_control_groups_system())
        .add_system(avoidance_system())
        .add_system(add_attack_commands_system())
        // Needed because a command could place a building using a command buffer, but the entity
        // reference wouldn't be valid until the commands in the buffer have been executed.
        .flush()
        .add_system(set_movement_paths_system())
        .add_system(reduce_cooldowns_system())
        .add_system(set_debug_pathfinding_start_system())
        // Cheese droplets.
        .add_system(spawn_cheese_droplets_system())
        .flush()
        .add_system(apply_gravity_system())
        .add_system(move_cheese_droplets_system())
        .add_system(move_units_system())
        .add_system(move_bullets_system())
        .add_system(apply_steering_system())
        .add_system(build_buildings_system())
        .add_system(firing_system())
        .add_system(apply_bullets_system())
        .flush()
        .add_system(handle_damaged_system());
}

pub fn add_rendering_systems(builder: &mut legion::systems::Builder) {
    builder
        .add_system(progress_animations_system())
        .add_system(progress_building_animations_system())
        // Rendering
        .add_system(render_bullets_system())
        .add_system(render_units_system())
        .add_system(render_selections_system())
        //.add_system(render_firing_ranges_system())
        .add_system(render_under_select_box_system())
        .add_system(render_drag_box_system())
        .add_system(render_command_paths_system())
        .add_system(render_ui_system())
        .add_system(render_health_bars_system())
        .add_system(render_unit_under_cursor_system())
        //.add_system(render_pathfinding_map_system())
        //.add_system(render_unit_paths_system())
        .add_system(render_debug_unit_pathfinding_system())
        .add_system(render_buildings_system())
        .add_system(render_building_plan_system())
        .add_system(render_cheese_droplets_system())
        //.add_system(debug_select_box_system())
        //.add_system(debug_specific_path_system())
        // Cleanup
        .flush()
        .add_system(cleanup_controls_system());
}

pub struct CheeseGuyser;

#[derive(Debug)]
pub struct Position(pub Vec2);
pub struct Facing(pub f32);
#[derive(PartialEq, Clone, Copy)]
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
        state: ActionState,
    },
    Build {
        target: Entity,
        state: ActionState,
    },
}

impl Command {
    fn new_attack(target: Entity, explicit: bool) -> Self {
        Self::Attack {
            target,
            explicit,
            first_out_of_range: true,
            state: ActionState::OutOfRange { path: Vec::new() },
        }
    }

    fn path(&self) -> Option<&Vec<Vec2>> {
        if let &Command::MoveTo { ref path, .. }
        | &Command::Attack {
            state: ActionState::OutOfRange { ref path },
            ..
        }
        | &Command::Build {
            state: ActionState::OutOfRange { ref path },
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
            state: ActionState::OutOfRange { ref mut path },
            ..
        }
        | &mut Command::Build {
            state: ActionState::OutOfRange { ref mut path },
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
pub enum ActionState {
    OutOfRange { path: Vec<Vec2> },
    InRange,
}

impl ActionState {
    fn is_out_of_range(&self) -> bool {
        matches!(self, Self::OutOfRange { .. })
    }
}

#[derive(Default)]
pub struct CommandQueue(VecDeque<Command>);

pub struct Health(pub u16);
pub struct BuildingCompleteness(pub u16);

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

pub struct Cooldown(pub f32);

#[derive(Copy, Clone, PartialEq)]
pub enum Building {
    Armoury,
    Pump,
}

struct BuildingStats {
    pub radius: f32,
    pub dimensions: Vec2,
    pub max_health: u16,
}

impl Building {
    fn stats(self) -> BuildingStats {
        match self {
            Self::Armoury => BuildingStats {
                radius: 6.0,
                dimensions: Vec2::new(6.0, 10.0),
                max_health: 500,
            },
            Self::Pump => BuildingStats {
                radius: 3.0,
                dimensions: Vec2::new(4.0, 4.0),
                max_health: 200,
            },
        }
    }

    fn parts(
        self,
        position: Vec2,
        side: Side,
        map: &mut Map,
    ) -> Option<(
        Position,
        MapHandle,
        Self,
        Radius,
        Selectable,
        Side,
        Health,
        BuildingCompleteness,
    )> {
        let BuildingStats {
            radius,
            dimensions,
            max_health,
        } = self.stats();

        let handle = map.insert(position, dimensions)?;

        Some((
            Position(position),
            handle,
            self,
            Radius(radius),
            Selectable,
            side,
            Health(1),
            BuildingCompleteness(1),
        ))
    }

    pub fn add_to_world_finished(
        self,
        position: Vec2,
        side: Side,
        world: &mut World,
        animations: &ModelAnimations,
        map: &mut Map,
    ) -> Option<Entity> {
        let mut parts = self.parts(position, side, map)?;
        let max_health = self.stats().max_health;
        parts.6 = Health(max_health);
        parts.7 = BuildingCompleteness(max_health);
        let entity = world.push(parts);

        if let Building::Pump = self {
            let mut entry = world.entry(entity).unwrap();

            entry.add_component(animations.pump.skin.clone());
            entry.add_component(AnimationState {
                animation: 0,
                time: 0.0,
                total_time: animations.pump.animations[0].total_time,
            });
        }

        Some(entity)
    }
}

#[derive(Copy, Clone)]
pub enum Unit {
    MouseMarine,
    Engineer,
}

pub struct UnitStats {
    pub max_health: u16,
    pub move_speed: f32,
    pub radius: f32,
    pub firing_range: f32,
    pub health_bar_height: f32,
}

enum MouseAnimation {
    Build = 0,
    Idle = 1,
    Shoot = 2,
    Walking = 3,
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
            Self::Engineer => UnitStats {
                max_health: 40,
                firing_range: 1.0,
                move_speed: 6.0,
                radius: 1.0,
                health_bar_height: 3.0,
            },
        }
    }

    pub fn add_to_world(
        self,
        world: &mut World,
        // This is only `None` when being run in a test
        animations: Option<&ModelAnimations>,
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
            Cooldown(0.0),
            FiringRange(firing_range),
            MoveSpeed(move_speed),
            Radius(radius),
            // Uncomment to debug movement.
            // MovementDebugging::default(),
        ));

        let mut entry = world.entry(entity).unwrap();

        if let Unit::Engineer = self {
            entry.add_component(CanBuild);
        }

        if let Some(animations) = animations {
            entry.add_component(animations.mouse.skin.clone());
            entry.add_component(AnimationState {
                animation: MouseAnimation::Idle as usize,
                time: 0.0,
                total_time: animations.mouse.animations[MouseAnimation::Idle as usize].total_time,
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
    pub top_left: Vec2,
    pub top_right: Vec2,
    pub bottom_left: Vec2,
    pub bottom_right: Vec2,
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

        ncollide2d::utils::is_point_in_triangle(
            &point,
            &top_left_point,
            &top_right_point,
            &bottom_left_point,
        ) || ncollide2d::utils::is_point_in_triangle(
            &point,
            &top_right_point,
            &bottom_left_point,
            &bottom_right_point,
        )
    }
}

fn vec2_to_ncollide_point(point: Vec2) -> ncollide2d::math::Point<f32> {
    ncollide2d::math::Point::new(point.x, point.y)
}

pub struct CheeseDropletPosition(Vec3);
pub struct CheeseDropletVelocity(Vec3);
pub struct CanBuild;
