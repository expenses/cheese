use crate::renderer::{
    Font, LineBuffers, ModelInstance, TextAlignment, TextBuffer, TitlescreenBuffer,
};
use crate::resources::{
    Camera, CursorIcon, DeltaTime, DpiScaling, Mode, MouseState, ScreenDimensions,
};
use legion::*;
use ultraviolet::{Mat4, Rotor3, Vec2, Vec3, Vec4};

const TITLE_POSITION: Vec2 = Vec2::new(0.5, 1.0 / 6.0);
const SKIRMISH_POSITION: Vec2 = Vec2::new(0.5, 3.0 / 4.0);
const QUIT_POSITION: Vec2 = Vec2::new(0.5, 3.25 / 4.0);

pub fn titlescreen_schedule() -> Schedule {
    Schedule::builder()
        .add_system(update_system())
        .add_system(handle_clicks_system())
        .add_system(render_text_system())
        //.add_system(render_click_regions_system())
        .add_system(crate::ecs::cleanup_controls_system())
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
    #[resource] mouse_state: &MouseState,
    #[resource] cursor_icon: &mut CursorIcon,
) {
    let screen_dimensions = screen_dimensions.as_vec();

    let colour = Vec4::new(0.867, 0.675, 0.086, 1.0);
    let selected_colour = colour * 0.5 + Vec4::one() * 0.5;

    text_buffer.render_text(
        TITLE_POSITION * screen_dimensions,
        "Cheese (working title :^))",
        Font::Title,
        1.5,
        dpi_scaling.0,
        TextAlignment::Center,
        colour,
    );

    for (text, position) in [("Skirmish", SKIRMISH_POSITION), ("Quit", QUIT_POSITION)].iter() {
        let center = *position * screen_dimensions;

        let (top_left, bottom_right) = text_selection_area(center, text, dpi_scaling.0);
        let selected = point_in_area(mouse_state.position, top_left, bottom_right);

        if selected {
            cursor_icon.0 = winit::window::CursorIcon::Hand;
        }

        text_buffer.render_text(
            center,
            text,
            Font::Title,
            1.0,
            dpi_scaling.0,
            TextAlignment::Center,
            if selected { selected_colour } else { colour },
        );
    }
}

#[legion::system]
fn render_click_regions(
    #[resource] line_buffers: &mut LineBuffers,
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
) {
    let screen_dimensions = screen_dimensions.as_vec();

    for (text, position) in [("Skirmish", SKIRMISH_POSITION), ("Quit", QUIT_POSITION)].iter() {
        let center = *position * screen_dimensions;

        let (top_left, bottom_right) = text_selection_area(center, text, dpi_scaling.0);
        line_buffers.draw_rect(top_left, bottom_right, dpi_scaling.0);
    }
}

#[legion::system]
fn handle_clicks(
    #[resource] screen_dimensions: &ScreenDimensions,
    #[resource] dpi_scaling: &DpiScaling,
    #[resource] mouse_state: &MouseState,
    #[resource] mode: &mut Mode,
    #[resource] camera: &mut Camera,
) {
    if !mouse_state.left_state.was_clicked() {
        return;
    }

    let screen_dimensions = screen_dimensions.as_vec();

    let center = SKIRMISH_POSITION * screen_dimensions;
    let (top_left, bottom_right) = text_selection_area(center, "Skirmish", dpi_scaling.0);
    if point_in_area(mouse_state.position, top_left, bottom_right) {
        *mode = Mode::Playing;
        *camera = Camera::default();
        return;
    }

    let center = QUIT_POSITION * screen_dimensions;
    let (top_left, bottom_right) = text_selection_area(center, "Quit", dpi_scaling.0);
    if point_in_area(mouse_state.position, top_left, bottom_right) {
        *mode = Mode::Quit;
    }
}

// Kinda hacky code to get a selection box around some text. Works well enough though.
fn text_selection_area(center: Vec2, text: &str, dpi_scaling: f32) -> (Vec2, Vec2) {
    let dimensions = Vec2::new(
        text.len() as f32 / 2.0 * Font::Title.scale() * dpi_scaling,
        Font::Title.scale() * dpi_scaling,
    );

    (center - dimensions / 2.0, center + dimensions / 2.0)
}

fn point_in_area(point: Vec2, top_left: Vec2, bottom_right: Vec2) -> bool {
    point.x >= top_left.x
        && point.y >= top_left.y
        && point.x <= bottom_right.x
        && point.y <= bottom_right.y
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
                    * Rotor3::from_rotation_between(Vec3::new(0.0, 1.0, 0.0), pos)
                        .into_matrix()
                        .into_homogeneous()
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
