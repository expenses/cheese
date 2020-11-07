use crate::ecs;
use ultraviolet::{Mat4, Vec2, Vec3, Vec4};

#[derive(Default)]
pub struct CameraControls {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub zoom_delta: f32,
}

#[derive(Default)]
pub struct RtsControls {
    pub shift_held: bool,
    pub stop_pressed: bool,
    pub mode: CommandMode,
}

#[derive(PartialEq, Debug)]
pub enum CommandMode {
    Normal,
    AttackMove,
}

impl Default for CommandMode {
    fn default() -> Self {
        Self::Normal
    }
}

pub struct Camera {
    pub position: Vec3,
    pub looking_at: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 20.0, 10.0),
            looking_at: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Camera {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.looking_at, Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn cast_ray(&self, mouse_position: Vec2, screen_dimensions: &ScreenDimensions) -> Vec2 {
        let &ScreenDimensions { width, height } = screen_dimensions;

        let x = (mouse_position.x / width as f32 * 2.0) - 1.0;
        let y = 1.0 - (mouse_position.y / height as f32 * 2.0);

        let clip = Vec4::new(x, y, -1.0, 1.0);

        let eye = crate::renderer::create_perspective_mat4(width, height).inversed() * clip;
        let eye = Vec4::new(eye.x, eye.y, -1.0, 0.0);
        let direction = (self.to_matrix().inversed() * eye).truncated().normalized() * 1.0;

        let ray = ncollide3d::query::Ray::new(
            ncollide3d::math::Point::new(self.position.x, self.position.y, self.position.z),
            ncollide3d::math::Vector::new(direction.x, direction.y, direction.z),
        );

        let toi = ncollide3d::query::ray_toi_with_plane(
            &ncollide3d::math::Point::new(0.0, 0.0, 0.0),
            &ncollide3d::math::Vector::new(0.0, 1.0, 0.0),
            &ray,
        )
        .unwrap();

        let contact = self.position + direction * toi;
        Vec2::new(contact.x, contact.z)
    }
}

pub struct ScreenDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Default, Debug)]
pub struct MouseState {
    pub position: Vec2,
    pub left_state: MouseButtonState,
    pub right_state: MouseButtonState,
}

#[derive(Debug, Clone)]
pub enum MouseButtonState {
    Dragging(Vec2),
    Dragged(Vec2),
    Up,
    Clicked,
    Down(u8, Vec2),
}

impl Default for MouseButtonState {
    fn default() -> Self {
        Self::Up
    }
}

impl MouseButtonState {
    pub fn update(&mut self, mouse: Vec2) {
        match *self {
            Self::Clicked => *self = Self::Up,
            Self::Down(ref mut frames, start) => {
                let drag = *frames > 2 && (*frames > 10
                    || (mouse.x - start.x).abs() > 10.0
                    || (mouse.y - start.y).abs() > 10.0);

                if drag {
                    *self = Self::Dragging(start)
                } else {
                    *frames += 1;
                }

            },
            Self::Dragged(_) => *self = Self::Up,
            Self::Up | Self::Dragging(_) => {}
        }
    }

    pub fn handle(&mut self, mouse: Vec2, pressed: bool) {
        if pressed {
            self.handle_down(mouse);
        } else {
            self.handle_up();
        }
    }

    fn handle_down(&mut self, mouse: Vec2) {
        *self = Self::Down(0, mouse)
    }

    fn handle_up(&mut self) {
        match *self {
            Self::Down(_, _) => *self = Self::Clicked,
            Self::Dragging(start) => *self = Self::Dragged(start),
            _ => *self = Self::Up,
        }
    }

    pub fn was_clicked(&self) -> bool {
        matches!(self, Self::Clicked)
    }

    pub fn is_being_dragged(&self) -> Option<Vec2> {
        if let Self::Dragging(start) = self {
            Some(*start)
        } else {
            None
        }
    }

    pub fn was_dragged(&self) -> Option<Vec2> {
        if let Self::Dragged(start) = self {
            Some(*start)
        } else {
            None
        }
    }
}

pub struct PlayerSide(pub ecs::Side);
pub struct DeltaTime(pub f32);
pub struct CursorIcon(pub winit::window::CursorIcon);
#[derive(Default)]
pub struct RayCastLocation(pub Vec2);
