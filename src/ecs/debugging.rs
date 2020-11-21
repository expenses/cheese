use super::{
    AnimationState, Building, CommandQueue, FiringRange, MovementDebugging, Position, Selected,
    Side,
};
use crate::assets::ModelAnimations;
use crate::pathfinding::Map;
use crate::renderer::{Lines3dBuffer, TorusBuffer, TorusInstance};
use crate::resources::{DebugControls, PlayerSide, RayCastLocation};
use legion::component;
use legion::systems::CommandBuffer;
use ultraviolet::{Vec2, Vec3, Vec4};

#[legion::system(for_each)]
#[filter(component::<Selected>())]
pub fn render_firing_ranges(
    position: &Position,
    firing_range: &FiringRange,
    side: &Side,
    #[resource] player_side: &PlayerSide,
    #[resource] torus_buffer: &mut TorusBuffer,
) {
    if *side != player_side.0 {
        return;
    }

    torus_buffer.toruses.push(TorusInstance {
        center: Vec3::new(position.0.x, 0.0, position.0.y),
        colour: Vec3::new(0.5, 0.0, 0.0),
        radius: firing_range.0,
    });
}

#[legion::system]
pub fn set_debug_pathfinding_start(
    #[resource] debug_controls: &mut DebugControls,
    #[resource] ray_cast_location: &RayCastLocation,
) {
    if debug_controls.set_pathfinding_start_pressed {
        debug_controls.pathfinding_start = ray_cast_location.0;
    }
}

#[legion::system]
pub fn spawn_debug_building(
    #[resource] debug_controls: &DebugControls,
    #[resource] ray_cast_location: &RayCastLocation,
    #[resource] animations: &ModelAnimations,
    #[resource] map: &mut Map,
    command_buffer: &mut CommandBuffer,
) {
    if debug_controls.spawn_building_pressed {
        if let Some((pos, handle, building, radius, selectable, side, health)) =
            Building::Pump.parts(ray_cast_location.0, Side::Purple, map)
        {
            let skin = animations.pump.skin.clone();
            let animation_state = AnimationState {
                animation: 0,
                time: 0.0,
                total_time: animations.pump.animations[0].total_time,
            };
            command_buffer.push((
                pos,
                handle,
                building,
                radius,
                selectable,
                side,
                health,
                skin,
                animation_state,
            ));
        }
    }
}

#[legion::system]
pub fn render_building_grid(#[resource] lines_3d_buffer: &mut Lines3dBuffer) {
    let size = 100;
    let colour = Vec4::new(1.0, 1.0, 1.0, 1.0);

    for n in -size..=size {
        let n = n as f32;
        let size = size as f32;

        lines_3d_buffer.draw_line(Vec2::new(n, -size), Vec2::new(n, size), 0.09, colour);
        lines_3d_buffer.draw_line(Vec2::new(-size, n), Vec2::new(size, n), 0.09, colour);
    }
}

#[legion::system]
pub fn render_pathfinding_map(
    #[resource] map: &Map,
    #[resource] lines_3d_buffer: &mut Lines3dBuffer,
) {
    for (a, b, constraint) in map.edges() {
        let colour = if constraint {
            Vec4::new(1.0, 0.0, 0.0, 1.0)
        } else {
            Vec4::new(1.0, 1.0, 1.0, 1.0)
        };

        lines_3d_buffer.draw_line(a, b, 0.1, colour);
    }
}

// There is a bug in the pathfinding code that means that units go out to the edge of the map and
// back in a very specific circumstance.
#[legion::system]
pub fn debug_specific_path(#[resource] map: &Map, #[resource] lines_3d_buffer: &mut Lines3dBuffer) {
    let start = Vec2 {
        x: -16.221794,
        y: 4.150668,
    };
    let end = Vec2 {
        x: -19.93689,
        y: 15.350003,
    };

    let mut debug_triangles = Vec::new();
    let mut debug_funnel_portals = Vec::new();
    if let Some(path) = map.pathfind(
        start,
        end,
        1.0,
        Some(&mut debug_triangles),
        Some(&mut debug_funnel_portals),
    ) {
        render_path(start, &path, lines_3d_buffer);
    }
    render_triangles(&debug_triangles, lines_3d_buffer);
    render_funnel_points(&debug_funnel_portals, lines_3d_buffer);
}

#[legion::system(for_each)]
#[filter(component::<Selected>())]
pub fn render_debug_unit_pathfinding(
    commands: &CommandQueue,
    movement_debugging: &MovementDebugging,
    #[resource] lines_3d_buffer: &mut Lines3dBuffer,
) {
    if let Some(path) = commands.0.front().and_then(|command| command.path()) {
        if path.len() > 1 {
            render_triangles(&movement_debugging.triangles, lines_3d_buffer);
            render_funnel_points(&movement_debugging.funnel_points, lines_3d_buffer);
            // Print out the start and end points of the path. Useful for reproducing.
            println!(
                "{:?} -> {:?}",
                movement_debugging.path_start, movement_debugging.path_end
            );
        }
    }
}

#[legion::system(for_each)]
pub fn render_unit_paths(
    position: &Position,
    commands: &CommandQueue,
    #[resource] lines_3d_buffer: &mut Lines3dBuffer,
) {
    if let Some(path) = commands.0.front().and_then(|command| command.path()) {
        render_path(position.0, path, lines_3d_buffer);
    }
}

fn render_triangles(triangles: &[(Vec2, Vec2)], lines_3d_buffer: &mut Lines3dBuffer) {
    let mut prev = None;
    for &(center, special) in triangles {
        if let Some((prev_center, prev_special)) = prev {
            lines_3d_buffer.draw_line(prev_center, center, 0.3, Vec4::new(0.25, 0.25, 0.25, 1.0));
            lines_3d_buffer.draw_line(prev_special, special, 1.5, Vec4::new(0.0, 0.0, 0.0, 1.0));
        }
        prev = Some((center, special));
    }
}

fn render_funnel_points(funnel_points: &[(Vec2, Vec2)], lines_3d_buffer: &mut Lines3dBuffer) {
    let mut prev = None;
    for &(left, right) in funnel_points {
        if let Some((prev_left, prev_right)) = prev {
            lines_3d_buffer.draw_line(prev_left, left, 1.0, Vec4::new(1.0, 1.0, 0.0, 1.0));
            lines_3d_buffer.draw_line(prev_right, right, 1.0, Vec4::new(0.0, 1.0, 1.0, 1.0));
        }

        prev = Some((left, right));
    }
}

fn render_path(mut prev: Vec2, path: &[Vec2], lines_3d_buffer: &mut Lines3dBuffer) {
    for &point in path {
        lines_3d_buffer.draw_line(prev, point, 0.5, Vec4::new(1.0, 0.0, 1.0, 1.0));
        prev = point;
    }
}
