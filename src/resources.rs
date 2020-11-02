
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
    pub shift: bool,
}

pub struct Camera {
	pub position: Vec3,
	pub looking_at: Vec3,
}

impl Camera {
	pub fn to_matrix(&self) -> Mat4 {
		Mat4::look_at(
			self.position,
			self.looking_at,
			Vec3::new(0.0, 1.0, 0.0)
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

		let ray = ncollide3d::query::Ray::new(
			ncollide3d::math::Point::new(self.position.x, self.position.y, self.position.z),
			ncollide3d::math::Vector::new(direction.x, direction.y, direction.z)
		);

		let toi = ncollide3d::query::ray_toi_with_plane(
			&ncollide3d::math::Point::new(0.0, 0.0, 0.0),
			&ncollide3d::math::Vector::new(0.0, 1.0, 0.0),
			&ray
		).unwrap();

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
    pub left_clicked: bool,
    pub right_clicked: bool,
}
