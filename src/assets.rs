use wgpu::util::DeviceExt;
use crate::renderer::{Vertex, TEXTURE_FORMAT};

pub struct Model {
	pub buffer: wgpu::Buffer,
	pub bind_group: wgpu::BindGroup,
	pub num_vertices: u32,
}

impl Model {
	pub fn load(
		obj_bytes: &[u8], image_bytes: &[u8], bind_group_layout: &wgpu::BindGroupLayout,
		device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder
	) -> anyhow::Result<Self> {
		let mut reader = std::io::BufReader::new(obj_bytes);
		let obj::ObjData { texture, normal, position, objects, .. } = obj::ObjData::load_buf(&mut reader)?;

		let vertices: Vec<_> = objects.into_iter()
			.flat_map(|object| object.groups)
			.flat_map(|group| group.polys)
			.flat_map(|polygon| {
				polygon.0
			})
			.map(|obj::IndexTuple(position_index, texture_index, normal_index)| {
				let texture_index = texture_index.unwrap();
				let normal_index = normal_index.unwrap();

				Vertex {
					position: position[position_index].into(),
					normal: normal[normal_index].into(),
					uv: texture[texture_index].into(),
				}
			})
			.collect();

		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&vertices),
			usage: wgpu::BufferUsage::VERTEX
		});

		let texture_bind_group = load_texture(image_bytes, bind_group_layout, device, encoder);

		Ok(Self {
			buffer, bind_group: texture_bind_group, num_vertices: vertices.len() as u32,
		})
	}
}

fn load_texture(
	bytes: &[u8], bind_group_layout: &wgpu::BindGroupLayout,
	device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder,
) -> wgpu::BindGroup {
	let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap()
		.into_rgba();

	let temp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Cheese load_texture buffer"),
		contents: &*image,
		usage: wgpu::BufferUsage::COPY_SRC,
	});

	let texture_extent = wgpu::Extent3d {
		width: image.width(),
		height: image.height(),
		depth: 1,
	};

	let texture = device.create_texture(&wgpu::TextureDescriptor {
		size: texture_extent,
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format: TEXTURE_FORMAT,
		usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
		label: Some("Cheese texture"),
	});

	encoder.copy_buffer_to_texture(
		wgpu::BufferCopyView {
			buffer: &temp_buf,
			layout: wgpu::TextureDataLayout {
				offset: 0,
				bytes_per_row: 4 * image.width(),
				rows_per_image: 0,
			}
		},
		wgpu::TextureCopyView {
			texture: &texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
		},
		texture_extent,
	);

	let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

	device.create_bind_group(&wgpu::BindGroupDescriptor {
		label: None,
		layout: bind_group_layout,
		entries: &[
			wgpu::BindGroupEntry {
				binding: 0,
				resource: wgpu::BindingResource::TextureView(&view),
			},
		],
	})
}
