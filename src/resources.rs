use crate::ecs;
use legion::Entity;
use std::collections::BTreeMap;
use ultraviolet::{Mat4, Vec2, Vec3, Vec4};
use winit::event::VirtualKeyCode;

#[derive(Default)]
pub struct CameraControls {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub zoom_delta: f32,
}

#[derive(Default, Debug)]
pub struct RtsControls {
    pub shift_held: bool,
    pub control_held: bool,
    pub stop_pressed: bool,
    pub mode: CommandMode,
    pub control_group_key_pressed: [bool; 10],
}

#[derive(Default, Debug)]
pub struct DebugControls {
    pub spawn_building_pressed: bool,
    pub pathfinding_start: Vec2,
    pub set_pathfinding_start_pressed: bool,
}

#[derive(PartialEq, Debug)]
pub enum CommandMode {
    Normal,
    AttackMove,
    Construct { building: ecs::Building },
    SetRecruitmentWaypoint,
}

impl Default for CommandMode {
    fn default() -> Self {
        Self::Normal
    }
}

pub struct Camera {
    pub distance: f32,
    pub looking_at: Vec2,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            distance: Self::ANGLE.mag(),
            looking_at: Vec2::new(0.0, 0.0),
        }
    }
}

impl Camera {
    const ANGLE: Vec3 = Vec3::new(0.0, 20.0, 10.0);

    fn looking_at_3(&self) -> Vec3 {
        Vec3::new(self.looking_at.x, 0.0, self.looking_at.y)
    }

    fn position(&self) -> Vec3 {
        self.looking_at_3() + Self::ANGLE.normalized() * self.distance
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::look_at(
            self.position(),
            self.looking_at_3(),
            Vec3::new(0.0, 1.0, 0.0),
        )
    }

    pub fn cast_ray(&self, mouse_position: Vec2, screen_dimensions: &ScreenDimensions) -> Vec2 {
        let &ScreenDimensions { width, height } = screen_dimensions;

        let x = (mouse_position.x / width as f32 * 2.0) - 1.0;
        let y = 1.0 - (mouse_position.y / height as f32 * 2.0);

        let clip = Vec4::new(x, y, -1.0, 1.0);

        let eye = crate::renderer::create_perspective_mat4(width, height).inversed() * clip;
        let eye = Vec4::new(eye.x, eye.y, -1.0, 0.0);
        let direction = (self.to_matrix().inversed() * eye).truncated().normalized() * 1.0;

        let position = self.position();
        let ray = ncollide3d::query::Ray::new(
            ncollide3d::math::Point::new(position.x, position.y, position.z),
            ncollide3d::math::Vector::new(direction.x, direction.y, direction.z),
        );

        let toi = ncollide3d::query::ray_toi_with_plane(
            &ncollide3d::math::Point::new(0.0, 0.0, 0.0),
            &ncollide3d::math::Vector::new(0.0, 1.0, 0.0),
            &ray,
        );

        match toi {
            Some(toi) => {
                let contact = self.position() + direction * toi;
                Vec2::new(contact.x, contact.z)
            }
            // The above ray cast can fail in odd cases such as where the window is minimized,
            // So let's just return the point the camera is centered on.
            None => self.looking_at,
        }
    }
}

pub struct ScreenDimensions {
    pub width: u32,
    pub height: u32,
}

impl ScreenDimensions {
    pub fn as_vec(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32)
    }
}

#[derive(Debug)]
pub struct MouseState {
    pub position: Vec2,
    pub left_state: MouseButtonState,
    pub right_state: MouseButtonState,
}

impl MouseState {
    pub fn new(screen_dimensions: &ScreenDimensions) -> Self {
        Self {
            // On osx, the window can take a while to get into place because it's doing some wierd animation thing.
            // While this is happening, the game is still running and if we set the mouse position to (0, 0) then
            // the camera will be going off into the top left corner the whole time. The obvious fix to this is to simply
            // have a title screen, but let's do things the hacky way for now and set the mouse position to the middle of the window
            // until it can start responding to events.
            position: Vec2::new(
                screen_dimensions.width as f32 / 2.0,
                screen_dimensions.height as f32 / 2.0,
            ),
            left_state: Default::default(),
            right_state: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MouseButtonState {
    Dragging(Vec2),
    Dragged(Vec2),
    Up,
    Clicked,
    Down(f32, Vec2),
}

impl Default for MouseButtonState {
    fn default() -> Self {
        Self::Up
    }
}

impl MouseButtonState {
    pub fn update(&mut self, delta_time: f32) {
        match *self {
            Self::Clicked => *self = Self::Up,
            Self::Down(ref mut time_down, start) => {
                let drag = *time_down > 0.1;

                if drag {
                    *self = Self::Dragging(start)
                } else {
                    *time_down += delta_time;
                }
            }
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
        *self = Self::Down(0.0, mouse)
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
pub struct RayCastLocation {
    pub pos: Vec2,
    pub snapped_to_guyser: Option<Entity>,
}
pub struct DpiScaling(pub f32);

#[derive(Default)]
pub struct ControlGroups(pub [Vec<legion::Entity>; 10]);

#[derive(Clone, Copy)]
pub enum Mode {
    Titlescreen,
    Playing,
    Quit,
}

pub struct Gravity(pub f32);
pub struct CheeseCoins(pub u32);
#[derive(Default)]
pub struct SelectedUnitsAbilities(pub BTreeMap<&'static ecs::Ability, Vec<Entity>>);

pub struct Keypress {
    pub code: Option<VirtualKeyCode>,
    pub scancode: u32,
    pub pressed: bool,
}

#[derive(Default)]
pub struct Keypresses(pub Vec<Keypress>);

#[derive(Debug)]
pub enum WinCondition {
    BuildN(u8, ecs::Building),
    DestroyAll,
}

#[derive(Debug)]
pub enum LoseCondition {
    LetAllUnitsDie,
}

#[derive(Debug)]
pub struct Objectives {
    pub win_conditions: Vec<WinCondition>,
    pub lose_conditions: Vec<LoseCondition>,
}

#[derive(Debug)]
pub enum PlayingState {
    Won,
    Lost,
    InProgress,
}

pub struct Settings {
    pub bloom: bool,
    pub shadow_resolution: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            bloom: true,
            shadow_resolution: 1024,
        }
    }
}
