use cgmath::{Matrix4, Quaternion, Vector3};

#[derive(Clone, Debug)]
pub struct Nodes {
    nodes: Vec<Node>,
    roots_indices: Vec<usize>,
    depth_first_taversal_indices: Vec<(usize, Option<usize>)>,
}

impl Nodes {
    pub fn from_gltf_nodes(gltf_nodes: gltf::iter::Nodes, scene: &gltf::Scene) -> Nodes {
        let roots_indices = scene.nodes().map(|n| n.index()).collect::<Vec<_>>();
        let node_count = gltf_nodes.len();
        let mut nodes = Vec::with_capacity(node_count);
        for node in gltf_nodes {
            let node_index = node.index();

            let (local_translation, local_rotation, local_scale) = node.transform().decomposed();
            let local_translation: Vector3<f32> = local_translation.into();
            // Different order!!!
            let [xr, yr, zr, wr] = local_rotation;
            let local_rotation = cgmath::Quaternion::new(wr, xr, yr, zr);
            let local_scale: Vector3<f32> = local_scale.into();

            let global_transform_matrix =
                compute_transform_matrix(local_translation, local_rotation, local_scale);
            let mesh_index = node.mesh().map(|m| m.index());
            let skin_index = node.skin().map(|s| s.index());
            let children_indices = node.children().map(|c| c.index()).collect::<Vec<_>>();
            let node = Node {
                local_translation,
                local_rotation,
                local_scale,
                global_transform_matrix,
                mesh_index,
                skin_index,
                children_indices,
            };
            nodes.insert(node_index, node);
        }

        let mut nodes = Nodes::new(nodes, roots_indices);
        // Derive the global transform
        nodes.transform(None);
        nodes
    }

    fn new(nodes: Vec<Node>, roots_indices: Vec<usize>) -> Self {
        let depth_first_taversal_indices = build_graph_run_indices(&roots_indices, &nodes);
        Self {
            roots_indices,
            nodes,
            depth_first_taversal_indices,
        }
    }
}

impl Nodes {
    pub fn transform(&mut self, global_transform: Option<Matrix4<f32>>) {
        for (index, parent_index) in &self.depth_first_taversal_indices {
            let parent_transform = parent_index
                .map(|id| {
                    let parent = &self.nodes[id];
                    parent.global_transform_matrix
                })
                .or(global_transform);

            if let Some(matrix) = parent_transform {
                let node = &mut self.nodes[*index];
                node.apply_transform(matrix);
            }
        }
    }
}

fn build_graph_run_indices(roots_indices: &[usize], nodes: &[Node]) -> Vec<(usize, Option<usize>)> {
    let mut indices = Vec::new();
    for root_index in roots_indices {
        build_graph_run_indices_rec(nodes, *root_index, None, &mut indices);
    }
    indices
}

fn build_graph_run_indices_rec(
    nodes: &[Node],
    node_index: usize,
    parent_index: Option<usize>,
    indices: &mut Vec<(usize, Option<usize>)>,
) {
    indices.push((node_index, parent_index));
    for child_index in &nodes[node_index].children_indices {
        build_graph_run_indices_rec(nodes, *child_index, Some(node_index), indices);
    }
}

impl Nodes {
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut [Node] {
        &mut self.nodes
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub local_translation: Vector3<f32>,
    pub local_rotation: Quaternion<f32>,
    pub local_scale: Vector3<f32>,
    global_transform_matrix: Matrix4<f32>,
    mesh_index: Option<usize>,
    skin_index: Option<usize>,
    children_indices: Vec<usize>,
}

impl Node {
    fn apply_transform(&mut self, transform: Matrix4<f32>) {
        let local_transform = compute_transform_matrix(
            self.local_translation,
            self.local_rotation,
            self.local_scale,
        );

        let new_tranform = transform * local_transform;
        self.global_transform_matrix = new_tranform;
    }
}

impl Node {
    pub fn transform(&self) -> Matrix4<f32> {
        self.global_transform_matrix
    }

    pub fn mesh_index(&self) -> Option<usize> {
        self.mesh_index
    }

    pub fn skin_index(&self) -> Option<usize> {
        self.skin_index
    }
}

fn compute_transform_matrix(
    translation: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
) -> Matrix4<f32> {
    let translation = Matrix4::from_translation(translation);
    let rotation = Matrix4::from(rotation);
    let scale = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
    translation * rotation * scale
}
