use ultraviolet::{Vec2, Vec3, Mat4};
use crate::renderer::{Instance, InstanceBuffers};

pub struct Position(pub Vec2);
pub struct Facing(pub f32);
pub enum Side {
    Green,
    Purple,
}
pub struct Selected;

#[legion::system(for_each)]
pub fn render_boxes(
    position: &Position, facing: &Facing, side: &Side, selected: Option<&Selected>,
    #[resource] buffers: &mut InstanceBuffers
) {
    let translation = Mat4::from_translation(Vec3::new(position.0.x, 0.0, position.0.y));
    let rotation = Mat4::from_rotation_y(facing.0);

    let instance = Instance {
        transform: rotation * translation,
        uv_flip: match side {
            Side::Green => 1.0,
            Side::Purple => -1.0,
        }
    };

	buffers.mice.push(instance);

    if selected.is_some() {
        buffers.selection_indicators.push(instance);
    }
}
