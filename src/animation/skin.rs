use super::node::{Node, Nodes};
use gltf::{iter::Skins as GltfSkins, Skin as GltfSkin};
use cgmath::{Matrix4, SquareMatrix};

// Must be kept in sync with the value in model.vert
pub const MAX_JOINTS_PER_MESH: usize = 512;

#[derive(Clone, Debug)]
pub struct Skin {
    joints: Vec<Joint>,
    pub nodes: Nodes,
}

impl Skin {
    pub fn load(gltf_skin: &gltf::Skin, nodes: Nodes, buffers: &[Vec<u8>]) -> Self {
        let joint_count = gltf_skin.joints().count();
        let inverse_bind_matrices = map_inverse_bind_matrices(gltf_skin, buffers);
        let node_ids = map_node_ids(gltf_skin);
    
        let joints = inverse_bind_matrices
            .iter()
            .zip(node_ids)
            .map(|(matrix, node_id)| Joint::new(*matrix, node_id))
            .collect::<Vec<_>>();
    
        Skin { joints, nodes }
    }

    /// Compute the joints matrices from the nodes matrices.
    pub fn update(&mut self) {
        self.nodes.transform(None);

        let nodes = self.nodes.nodes();

        self.joints
            .iter_mut()
            .for_each(|j| j.compute_matrix(nodes));
    }
}

impl Skin {
    pub fn joints(&self) -> &[Joint] {
        &self.joints
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Joint {
    matrix: Matrix4<f32>,
    inverse_bind_matrix: Matrix4<f32>,
    node_id: usize,
}

impl Joint {
    fn new(inverse_bind_matrix: Matrix4<f32>, node_id: usize) -> Self {
        Joint {
            matrix: Matrix4::identity(),
            inverse_bind_matrix,
            node_id,
        }
    }

    fn compute_matrix(&mut self, nodes: &[Node]) {
        let node_transform = nodes[self.node_id].transform();

        self.matrix = node_transform * self.inverse_bind_matrix;
    }
}

impl Joint {
    pub fn matrix(&self) -> Matrix4<f32> {
        self.matrix
    }
}

fn map_inverse_bind_matrices(gltf_skin: &GltfSkin, data: &[Vec<u8>]) -> Vec<Matrix4<f32>> {
    let iter = gltf_skin
        .reader(|buffer| Some(&data[buffer.index()]))
        .read_inverse_bind_matrices()
        .expect("IBM reader not found for skin");
    iter.map(Matrix4::from).collect::<Vec<_>>()
}

fn map_node_ids(gltf_skin: &GltfSkin) -> Vec<usize> {
    gltf_skin
        .joints()
        .map(|node| node.index())
        .collect::<Vec<_>>()
}
