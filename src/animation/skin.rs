// This file was originally copied from gltf-viewer-rs:
// https://github.com/adrien-ben/gltf-viewer-rs/blob/master/model/src/skin.rs

use super::node::{Node, Nodes};
use ultraviolet::Mat4;

#[derive(Clone, Debug, Default)]
pub struct Skin {
    pub joints: Vec<Joint>,
    pub nodes: Nodes,
}

impl Skin {
    pub fn load(
        gltf_skin: &gltf::Skin,
        gltf_nodes: gltf::iter::Nodes,
        scene: &gltf::Scene,
        buffers: &[Vec<u8>],
    ) -> Self {
        let nodes = Nodes::from_gltf_nodes(gltf_nodes, scene);

        let inverse_bind_matrices: Vec<_> = gltf_skin
            .reader(|buffer| Some(&buffers[buffer.index()]))
            .read_inverse_bind_matrices()
            .unwrap()
            .map(|mat| mat.into())
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
        self.nodes.transform();

        let nodes = self.nodes.nodes();

        self.joints.iter_mut().for_each(|j| j.compute_matrix(nodes));
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Joint {
    pub matrix: Mat4,
    inverse_bind_matrix: Mat4,
    node_id: usize,
}

impl Joint {
    fn new(inverse_bind_matrix: Mat4, node_id: usize) -> Self {
        Joint {
            matrix: Mat4::identity(),
            inverse_bind_matrix,
            node_id,
        }
    }

    fn compute_matrix(&mut self, nodes: &[Node]) {
        let node_transform = nodes[self.node_id].global_transform;
        self.matrix = node_transform * self.inverse_bind_matrix;
    }
}
