use crate::renderer::{Font, LineBuffers, ModelInstance, TextBuffer, TitlescreenBuffer};
use crate::resources::{Camera, DeltaTime, DpiScaling, ScreenDimensions};
use legion::*;
use ultraviolet::{Mat4, Vec2, Vec3, Vec4};

pub fn titlescreen_schedule() -> Schedule {
    Schedule::builder()
        .add_system(update_system())
        .add_system(render_text_system())
        .add_system(render_click_regions_system())
        .build()
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
    let moon_position = Vec3::new(0.0, 0.0, 5.0);

    moon.rotation += 0.1 * delta_time.0;
    titlescreen_buffer.moon.write(ModelInstance {
        transform: Mat4::from_translation(moon_position) * Mat4::from_rotation_y(moon.rotation),
        flat_colour: Vec4::one(),
    });
    *camera = Camera {
        position: Vec3::new(0.0, 0.0, 0.0),
        looking_at: moon_position,
    };
}

#[legion::system]
fn render_text(
    #[resource] text_buffer: &mut TextBuffer,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
) {
    let x = screen_dimensions.width as f32 / 2.0;
    let y = screen_dimensions.height as f32 / 6.0;

    let colour = [0.867, 0.675, 0.086, 1.0];

    text_buffer.render_text(
        (x, y),
        "Cheese (working title :^))",
        Font::Title,
        1.5,
        dpi_scaling.0,
        true,
        colour,
    );

    let y = screen_dimensions.height as f32 * 3.0 / 4.0;

    text_buffer.render_text(
        (x, y),
        "Skirmish",
        Font::Title,
        1.0,
        dpi_scaling.0,
        true,
        colour,
    );

    let y = screen_dimensions.height as f32 * 3.25 / 4.0;

    text_buffer.render_text(
        (x, y),
        "Quit",
        Font::Title,
        dpi_scaling.0,
        1.0,
        true,
        colour,
    );
}

#[legion::system]
fn render_click_regions(
    #[resource] line_buffers: &mut LineBuffers,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
) {
    let center = Vec2::new(
        screen_dimensions.width as f32 / 2.0,
        screen_dimensions.height as f32 * 3.0 / 4.0,
    );

    let dimensions = Vec2::new(
        "Skirmish".len() as f32 / 2.0 * Font::Title.scale() * dpi_scaling.0,
        Font::Title.scale() * dpi_scaling.0,
    );

    line_buffers.draw_rect(
        center - dimensions / 2.0,
        center + dimensions / 2.0,
        dpi_scaling.0,
    );

    let center = Vec2::new(
        screen_dimensions.width as f32 / 2.0,
        screen_dimensions.height as f32 * 3.25 / 4.0,
    );

    let dimensions = Vec2::new(
        "Quit".len() as f32 / 2.0 * Font::Title.scale() * dpi_scaling.0,
        Font::Title.scale() * dpi_scaling.0,
    );

    line_buffers.draw_rect(
        center - dimensions / 2.0,
        center + dimensions / 2.0,
        dpi_scaling.0,
    );
}

pub fn create_stars<R: rand::Rng>(rng: &mut R) -> Vec<ModelInstance> {
    (0..2000)
        .map(|_| {
            let pos = uniform_sphere_distribution_from_coords(
                rng.gen_range(0.0, 1.0),
                // Only produce stars in the hemisphere in front of the camera.
                rng.gen_range(0.0, 0.5),
            ) * 10.0;

            ModelInstance {
                transform: Mat4::from_translation(pos)
                    // Not sure why we're inverting this or anything.
                    * Mat4::look_at(Vec3::new(0.0, 0.0, 0.0), pos, Vec3::new(0.0, 1.0, 0.0))
                        .inversed()
                    * Mat4::from_scale(rng.gen_range(0.01, 0.05)),
                flat_colour: Vec4::one(),
            }
        })
        .collect()
}

// http://corysimon.github.io/articles/uniformdistn-on-sphere/
// I copied this function from a previous project I was working on a while ago.
// I think technically the x and y arguments should be switched because we're using
// a Y-up coordinate system but whatever.
pub fn uniform_sphere_distribution_from_coords(x: f64, y: f64) -> Vec3 {
    use std::f64::consts::PI;

    let theta = 2.0 * PI * x;
    let phi = (1.0 - 2.0 * y).acos();

    Vec3::new(
        (phi.sin() * theta.cos()) as f32,
        (phi.sin() * theta.sin()) as f32,
        phi.cos() as f32,
    )
}
