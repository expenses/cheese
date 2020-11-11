use ultraviolet::{Vec3, Mat4};
use std::collections::HashMap;

pub struct JointTree {
    inner: Vec<(Mat4, Vec<usize>)>,
}

impl JointTree {
    pub fn from_skin(skin: &gltf::Skin, x: &[Mat4]) -> Self {
        // Pre-create vec.
        let mut vec = vec![(Mat4::identity(), Vec::new()); skin.joints().count()];

        let mut mapping: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

        skin.joints().enumerate().for_each(|(i, joint)| {
            mapping.insert(joint.index(), i);
        });

        skin.joints().enumerate().for_each(|(i, joint)| {
            for child in joint.children() {
                vec[i].1.push(mapping[&child.index()]);
            }
        });

        for (i, x) in vec.iter().enumerate() {
            println!("{}: {:?}", i, x.1);
        }

        Self {
            inner: vec,
        }
    }

    pub fn apply_poses(&mut self, poses: &[Mat4], ibt: &[Mat4]) {
        let mut stack = vec![(Mat4::identity(), 0)];

        while let Some((parent_transform, index)) = stack.pop() {
            let transform = parent_transform * poses[index];
            self.inner[index].0 = transform;
            for child in self.inner[index].1.iter() {
                stack.push((transform, *child));
            }
            self.inner[index].0 = self.inner[index].0 * ibt[index];
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn as_vec(&self, inverse_bind_matrices: &[Mat4]) -> Vec<Mat4> {
        (0.. self.len())
            //.zip(inverse_bind_matrices.iter())
            .map(|id| {
                self.inner[id].0
            }).collect()
    }
}

#[derive(Clone)]
pub struct AnimationTransform {
    pub translation: Vec3,
    pub rotation: cgmath::Quaternion<f32>,
}

fn cgmath_matrix4_to_mat4(quat: cgmath::Quaternion<f32>) -> Mat4 {
    let euler: cgmath::Euler<cgmath::Rad<f32>> = quat.into();
    let matrix: cgmath::Matrix4<f32> = euler.into();
    let raw: [[f32; 4]; 4] = matrix.into();
    raw.into()
}

pub struct Animation {
    pub inputs: Vec<f32>,
    // First Vec: joints. Second Vec: transforms.
    pub outputs: Vec<Vec<AnimationTransform>>,
}

impl Animation {
    pub fn interpolate(&self, time: f32, joint_tree: &mut crate::animation::JointTree, ibt: &[Mat4]) {
        use cgmath::InnerSpace;

        let mut first_index = 0;
        let mut second_index = 0;

        for (i, input_time) in self.inputs.iter().cloned().enumerate().skip(1) {
            if input_time > time {
                first_index = i - 1;
                second_index = i;
                break;
            }
        }

        let first_value = self.inputs[first_index];
        let second_value = self.inputs[second_index];
        let interp_factor = map_value(time, first_value, second_value, 0.0, 1.0);

        let mut poses = Vec::new();


        for i in 0 .. joint_tree.len() {
            let first = &self.outputs[i][first_index];
            let second = &self.outputs[i][second_index];

            if i > 0 {
                poses.push(Mat4::identity());
                continue;
            }

            let first_quat = first.rotation;
            let second_quat = second.rotation;
            let rotation = first_quat.slerp(second_quat, interp_factor).normalize();
            let rotation = cgmath_matrix4_to_mat4(rotation.into());

            let translation = first.translation * (1.0 - interp_factor) +
                second.translation * interp_factor;
            let translation = Mat4::from_translation(translation);

            let transform = rotation * translation;
            poses.push(transform);
        }

        /*assert_eq!(joint_tree.len(), self.outputs.len());


        for i in 0 .. joint_tree.len() {
            let rotation = cgmath_matrix4_to_mat4(self.outputs[i][0].rotation);

            let translation = Mat4::from_translation(self.outputs[i][0].translation);

            let transform = translation * rotation;
            poses.push(transform);
        }*/

        joint_tree.apply_poses(&poses, ibt);
    }
}

fn map_value(value: f32, start_i: f32, end_i: f32, start_o: f32, end_o: f32) -> f32 {
	start_o + (end_o - start_o) * ((value - start_i) / (end_i - start_i))
}
