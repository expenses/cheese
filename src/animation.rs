use ultraviolet::Mat4;
use std::collections::HashMap;

pub struct JointTree {
    inner: indextree::Arena<Mat4>,
    ids: Vec<indextree::NodeId>,
}

impl JointTree {
    // This is hoenstly not very well written at all, but I needed something that worked quickly so
    // *shrug*.
    pub fn from_skin(skin: &gltf::Skin) -> Self {
        let mut tree = indextree::Arena::new();

        // There should be a way to tidy this up and get rid of this.
        let mut gltf_indices_to_tree_ids = HashMap::new();
        let mut ids = Vec::new();

        skin.joints().for_each(|joint| {
            let id = tree.new_node(joint.transform().matrix().into());
            gltf_indices_to_tree_ids.insert(joint.index(), id);
            ids.push(id);
        });

        for joint in skin.joints() {
            let joint_id = gltf_indices_to_tree_ids[&joint.index()];

            for child in joint.children() {
                let child_id = gltf_indices_to_tree_ids[&child.index()];
                joint_id.append(child_id, &mut tree);
            }
        }

        Self {
            inner: tree, ids
        }
    }

    pub fn set_local_transform(&mut self, index: usize, transform: Mat4) {
        let mut node = self.inner.get_mut(self.ids[index]).unwrap();
        *node.get_mut() = transform;
    }

    fn get_global_transform(&self, id: indextree::NodeId) -> Mat4 {
        let mut transform = Mat4::identity();

        let mut next = Some(id);

        while let Some(id) = next {
            let node = self.inner.get(id).unwrap();
            transform = *node.get() * transform;
            next = node.parent();
        }

        transform
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn as_vec(&self, inverse_bind_matrices: &[Mat4]) -> Vec<Mat4> {
        self.ids.iter()
            .zip(inverse_bind_matrices.iter())
            .map(|(&id, &ix)| {
                self.get_global_transform(id) * ix
            }).collect()
    }
}
