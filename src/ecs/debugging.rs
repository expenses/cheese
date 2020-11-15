use super::Building;
use crate::pathfinding::Map;
use crate::renderer::Lines3dBuffer;
use crate::resources::{DebugControls, RayCastLocation};
use legion::systems::CommandBuffer;
use ultraviolet::{Vec2, Vec4};

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
    #[resource] map: &mut Map,
    command_buffer: &mut CommandBuffer,
) {
    if debug_controls.spawn_building_pressed {
        if let Some(building) = Building::new(ray_cast_location.0, Vec2::new(6.0, 10.0), map) {
            command_buffer.push((building,));
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
    #[resource] debug_controls: &DebugControls,
    #[resource] ray_cast_location: &RayCastLocation,
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

    let mut debug_triangles = Vec::new();
    //let mut debug_funnel_points = Vec::new();

    if let Some(path) = map.pathfind(
        debug_controls.pathfinding_start,
        ray_cast_location.0,
        1.0,
        Some(&mut debug_triangles),
        None, // Some(&mut debug_funnel_points),
    ) {
        let mut prev = debug_controls.pathfinding_start;

        for point in path {
            lines_3d_buffer.draw_line(prev, point, 0.2, Vec4::new(0.0, 1.0, 0.0, 1.0));
            prev = point;
        }
    }

    let mut prev = None;
    for (center, special) in debug_triangles {
        if let Some((prev_center, prev_special)) = prev {
            lines_3d_buffer.draw_line(prev_center, center, 0.3, Vec4::new(0.25, 0.25, 0.25, 1.0));
            lines_3d_buffer.draw_line(prev_special, special, 1.5, Vec4::new(0.0, 0.0, 0.0, 1.0));
        }
        prev = Some((center, special));
    }

    /*
    let mut prev = None;

    for (left, right) in debug_funnel_points {
        if let Some((prev_left, prev_right)) = prev {
            lines_3d_buffer.draw_line(prev_left, left, 1.0, Vec4::new(1.0, 1.0, 0.0, 1.0));
            lines_3d_buffer.draw_line(prev_right, right, 1.0, Vec4::new(0.0, 1.0, 1.0, 1.0));
        }

        prev = Some((left, right));
    }
    */
}
