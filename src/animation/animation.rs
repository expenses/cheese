// This file was originally copied from gltf-viewer-rs:
// https://github.com/adrien-ben/gltf-viewer-rs/blob/master/model/src/animation.rs

use super::skin::Skin;
use cgmath::{Quaternion, InnerSpace};
use gltf::{
    animation::{
        iter::Channels,
        util::{ReadOutputs, Reader},
        Channel as GltfChannel, Interpolation, Property,
    },
    buffer::Buffer,
    iter::Animations as GltfAnimations,
    Animation as GltfAnimation,
};
use std::cmp::Ordering;
use ultraviolet::{Lerp, Vec3};

/// slerp from cgmath is bugged.
///
/// This algorithm is suggested in the cgmath issue about slerp
/// https://github.com/rustgd/cgmath/issues/300
pub fn slerp(left: Quaternion<f32>, right: Quaternion<f32>, amount: f32) -> Quaternion<f32> {
    let num2;
    let num3;
    let num = amount;
    let mut num4 = (((left.v.x * right.v.x) + (left.v.y * right.v.y)) + (left.v.z * right.v.z))
        + (left.s * right.s);
    let mut flag = false;
    if num4 < 0.0 {
        flag = true;
        num4 = -num4;
    }
    if num4 > 0.999_999 {
        num3 = 1.0 - num;
        num2 = if flag { -num } else { num };
    } else {
        let num5 = num4.acos();
        let num6 = 1.0 / num5.sin();
        num3 = ((1.0 - num) * num5).sin() * num6;
        num2 = if flag {
            -(num * num5).sin() * num6
        } else {
            (num * num5).sin() * num6
        };
    }
    Quaternion::new(
        (num3 * left.s) + (num2 * right.s),
        (num3 * left.v.x) + (num2 * right.v.x),
        (num3 * left.v.y) + (num2 * right.v.y),
        (num3 * left.v.z) + (num2 * right.v.z),
    )
}

trait Interpolate: Copy {
    fn linear(self, other: Self, amount: f32) -> Self;

    fn cubic_spline(
        source: [Self; 3],
        source_time: f32,
        target: [Self; 3],
        target_time: f32,
        current_time: f32,
    ) -> Self;
}

impl Interpolate for Vec3 {
    fn linear(self, other: Self, amount: f32) -> Self {
        self.lerp(other, amount)
    }

    fn cubic_spline(
        source: [Self; 3],
        source_time: f32,
        target: [Self; 3],
        target_time: f32,
        amount: f32,
    ) -> Self {
        let t = amount;
        let p0 = source[1];
        let m0 = (target_time - source_time) * source[2];
        let p1 = target[1];
        let m1 = (target_time - source_time) * target[0];

        (2.0 * t * t * t - 3.0 * t * t + 1.0) * p0
            + (t * t * t - 2.0 * t * t + t) * m0
            + (-2.0 * t * t * t + 3.0 * t * t) * p1
            + (t * t * t - t * t) * m1
    }
}

impl Interpolate for Quaternion<f32> {
    fn linear(self, other: Self, amount: f32) -> Self {
        slerp(self, other, amount)
    }

    fn cubic_spline(
        source: [Self; 3],
        source_time: f32,
        target: [Self; 3],
        target_time: f32,
        amount: f32,
    ) -> Self {
        let t = amount;
        let p0 = source[1];
        let m0 = (target_time - source_time) * source[2];
        let p1 = target[1];
        let m1 = (target_time - source_time) * target[0];

        let result = (2.0 * t * t * t - 3.0 * t * t + 1.0) * p0
            + (t * t * t - 2.0 * t * t + t) * m0
            + (-2.0 * t * t * t + 3.0 * t * t) * p1
            + (t * t * t - t * t) * m1;

        result.normalize()
    }
}

#[derive(Debug)]
struct Channel<T> {
    interpolation: Interpolation,
    times: Vec<f32>,
    values: Vec<T>,
    node_index: usize,
}

impl<T> Channel<T> {
    fn get_max_time(&self) -> f32 {
        self.times.last().copied().unwrap_or(0.0)
    }
}

impl<T: Interpolate> Channel<T> {
    fn sample(&self, t: f32) -> Option<(usize, T)> {
        let index = {
            let mut index = None;
            for i in 0..(self.times.len() - 1) {
                let previous = self.times[i];
                let next = self.times[i + 1];
                if t >= previous && t < next {
                    index = Some(i);
                    break;
                }
            }
            index
        };

        index.map(|i| {
            let previous_time = self.times[i];
            let next_time = self.times[i + 1];
            let delta = next_time - previous_time;
            let from_start = t - previous_time;
            let factor = from_start / delta;

            let i = match self.interpolation {
                Interpolation::Step => self.values[i],
                Interpolation::Linear => {
                    let previous_value = self.values[i];
                    let next_value = self.values[i + 1];

                    previous_value.linear(next_value, factor)
                }
                Interpolation::CubicSpline => {
                    let previous_values = [
                        self.values[i * 3],
                        self.values[i * 3 + 1],
                        self.values[i * 3 + 2],
                    ];
                    let next_values = [
                        self.values[i * 3 + 3],
                        self.values[i * 3 + 4],
                        self.values[i * 3 + 5],
                    ];
                    Interpolate::cubic_spline(
                        previous_values,
                        previous_time,
                        next_values,
                        next_time,
                        factor,
                    )
                }
            };

            (self.node_index, i)
        })
    }
}

#[derive(Debug)]
pub struct Animation {
    pub total_time: f32,
    translation_channels: Vec<Channel<Vec3>>,
    rotation_channels: Vec<Channel<Quaternion<f32>>>,
    scale_channels: Vec<Channel<Vec3>>,
}

impl Animation {
    /// Update nodes' transforms from animation data.
    ///
    /// Returns true if any nodes was updated.
    pub fn animate(&self, skin: &mut Skin, time: f32) {
        let (translations, rotations, scale) = self.sample(time);
        translations.for_each(|(node_index, translation)| {
            skin.nodes.nodes_mut()[node_index].local_translation = translation;
        });
        rotations.for_each(|(node_index, rotation)| {
            skin.nodes.nodes_mut()[node_index].local_rotation = rotation;
        });
        scale.for_each(|(node_index, scale)| {
            skin.nodes.nodes_mut()[node_index].local_scale = scale;
        });

        skin.update();
    }

    fn sample(
        &self,
        t: f32,
    ) -> (
        impl Iterator<Item = (usize, Vec3)> + '_,
        impl Iterator<Item = (usize, Quaternion<f32>)> + '_,
        impl Iterator<Item = (usize, Vec3)> + '_,
    ) {
        (
            self.translation_channels
                .iter()
                .filter_map(move |tc| tc.sample(t)),
            self.rotation_channels
                .iter()
                .filter_map(move |tc| tc.sample(t)),
            self.scale_channels
                .iter()
                .filter_map(move |tc| tc.sample(t)),
        )
    }
}

pub fn load_animations(gltf_animations: GltfAnimations, data: &[Vec<u8>]) -> Vec<Animation> {
    gltf_animations
        .map(|a| map_animation(&a, data))
        .collect::<Vec<_>>()
}

fn map_animation(gltf_animation: &GltfAnimation, data: &[Vec<u8>]) -> Animation {
    let translation_channels = map_translation_channels(gltf_animation.channels(), data);
    let rotation_channels = map_rotation_channels(gltf_animation.channels(), data);
    let scale_channels = map_scale_channels(gltf_animation.channels(), data);

    let max_translation_time = translation_channels
        .iter()
        .map(Channel::get_max_time)
        .max_by(|c0, c1| c0.partial_cmp(&c1).unwrap_or(Ordering::Equal))
        .unwrap_or(0.0);
    let max_rotation_time = rotation_channels
        .iter()
        .map(Channel::get_max_time)
        .max_by(|c0, c1| c0.partial_cmp(&c1).unwrap_or(Ordering::Equal))
        .unwrap_or(0.0);
    let max_scale_time = scale_channels
        .iter()
        .map(Channel::get_max_time)
        .max_by(|c0, c1| c0.partial_cmp(&c1).unwrap_or(Ordering::Equal))
        .unwrap_or(0.0);

    let total_time = *[max_translation_time, max_rotation_time, max_scale_time]
        .iter()
        .max_by(|c0, c1| c0.partial_cmp(&c1).unwrap_or(Ordering::Equal))
        .unwrap_or(&0.0);

    Animation {
        total_time,
        translation_channels,
        rotation_channels,
        scale_channels,
    }
}

fn map_translation_channels(gltf_channels: Channels, data: &[Vec<u8>]) -> Vec<Channel<Vec3>> {
    gltf_channels
        .filter(|c| c.target().property() == Property::Translation)
        .filter_map(|c| map_translation_channel(&c, data))
        .collect::<Vec<_>>()
}

fn map_translation_channel(gltf_channel: &GltfChannel, data: &[Vec<u8>]) -> Option<Channel<Vec3>> {
    let gltf_sampler = gltf_channel.sampler();
    if let Property::Translation = gltf_channel.target().property() {
        let reader = gltf_channel.reader(|buffer| Some(&data[buffer.index()]));
        let times = read_times(&reader);
        let output = read_translations(&reader);
        Some(Channel {
            interpolation: gltf_sampler.interpolation(),
            times,
            values: output,
            node_index: gltf_channel.target().node().index(),
        })
    } else {
        None
    }
}

fn map_rotation_channels(
    gltf_channels: Channels,
    data: &[Vec<u8>],
) -> Vec<Channel<Quaternion<f32>>> {
    gltf_channels
        .filter(|c| c.target().property() == Property::Rotation)
        .filter_map(|c| map_rotation_channel(&c, data))
        .collect::<Vec<_>>()
}

fn map_rotation_channel(
    gltf_channel: &GltfChannel,
    data: &[Vec<u8>],
) -> Option<Channel<Quaternion<f32>>> {
    let gltf_sampler = gltf_channel.sampler();
    if let Property::Rotation = gltf_channel.target().property() {
        let reader = gltf_channel.reader(|buffer| Some(&data[buffer.index()]));
        let times = read_times(&reader);
        let output = read_rotations(&reader);
        Some(Channel {
            interpolation: gltf_sampler.interpolation(),
            times,
            values: output,
            node_index: gltf_channel.target().node().index(),
        })
    } else {
        None
    }
}

fn map_scale_channels(gltf_channels: Channels, data: &[Vec<u8>]) -> Vec<Channel<Vec3>> {
    gltf_channels
        .filter(|c| c.target().property() == Property::Scale)
        .filter_map(|c| map_scale_channel(&c, data))
        .collect::<Vec<_>>()
}

fn map_scale_channel(gltf_channel: &GltfChannel, data: &[Vec<u8>]) -> Option<Channel<Vec3>> {
    let gltf_sampler = gltf_channel.sampler();
    if let Property::Scale = gltf_channel.target().property() {
        let reader = gltf_channel.reader(|buffer| Some(&data[buffer.index()]));
        let times = read_times(&reader);
        let output = read_scales(&reader);
        Some(Channel {
            interpolation: gltf_sampler.interpolation(),
            times,
            values: output,
            node_index: gltf_channel.target().node().index(),
        })
    } else {
        None
    }
}

fn read_times<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<f32>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    reader.read_inputs().map_or(vec![], |times| times.collect())
}

fn read_translations<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<Vec3>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_outputs()
        .map_or(vec![], |outputs| match outputs {
            ReadOutputs::Translations(translations) => translations.map(Vec3::from).collect(),
            _ => vec![],
        })
}

fn read_scales<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<Vec3>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_outputs()
        .map_or(vec![], |outputs| match outputs {
            ReadOutputs::Scales(scales) => scales.map(Vec3::from).collect(),
            _ => vec![],
        })
}

fn read_rotations<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<Quaternion<f32>>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_outputs()
        .map_or(vec![], |outputs| match outputs {
            ReadOutputs::Rotations(scales) => scales
                .into_f32()
                .map(|r| Quaternion::new(r[3], r[0], r[1], r[2]))
                .collect(),
            _ => vec![],
        })
}
