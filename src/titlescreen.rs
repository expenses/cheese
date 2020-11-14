use crate::renderer::{ModelInstance, TitlescreenBuffer};
use crate::resources::{Camera, DeltaTime};
use legion::*;
use ultraviolet::{Mat4, Vec3, Vec4};

pub fn titlescreen_schedule() -> Schedule {
    Schedule::builder().add_system(update_system()).build()
}

#[derive(Default)]
pub struct TitlescreenMoon {
    rotation: f32,
}

#[legion::system]
fn update(
    #[resource] moon: &mut TitlescreenMoon,
    #[resource] delta_time: &DeltaTime,
    #[resource] titlescreen_buffer: &mut TitlescreenBuffer,
    #[resource] camera: &mut Camera,
) {
    moon.rotation += 0.1 * delta_time.0;
    titlescreen_buffer.moon.write(ModelInstance {
        transform: Mat4::from_rotation_y(moon.rotation),
        flat_colour: Vec4::one(),
    });
    *camera = Camera {
        position: Vec3::new(0.0, 0.0, -5.0),
        looking_at: Vec3::new(0.0, 0.0, 0.0),
    };
}
