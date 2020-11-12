use super::node::{Node, Nodes};
use cgmath::{Matrix4, SquareMatrix};

#[derive(Clone, Debug)]
pub struct Skin {
    joints: Vec<Joint>,
    pub nodes: Nodes,
}

impl Skin {
    pub fn load(gltf_skin: &gltf::Skin, nodes: Nodes, buffers: &[Vec<u8>]) -> Self {
        let joint_count = gltf_skin.joints().count();
        let inverse_bind_matrices: Vec<_> = gltf_skin
            .reader(|buffer| Some(&buffers[buffer.index()]))
            .read_inverse_bind_matrices()
            .unwrap()
            .map(|mat| Matrix4::from(mat))
            .collect();

        let node_ids = gltf_skin
            .joints()
            .map(|node| node.index())
            .collect::<Vec<_>>();

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

        self.joints.iter_mut().for_each(|j| j.compute_matrix(nodes));
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

    pub fn matrix(&self) -> Matrix4<f32> {
        self.matrix
    }
}
