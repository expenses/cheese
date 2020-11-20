#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;
layout(location = 3) in vec4 joint_indices;
layout(location = 4) in vec4 joint_weights;

layout(location = 5) in vec4 flat_colour;
layout(location = 6) in mat4 transform;

layout(set = 0, binding = 0) uniform Light {
    mat4 projection_view;
};

layout(set = 1, binding = 0) readonly buffer Joints {
	mat4 joints[];
};

layout(set = 1, binding = 1) uniform JointUniforms {
    uint num_joints;
};

void main() {
    uint joint_offset = gl_InstanceIndex * num_joints;

    // Calculate skinned matrix from weights and joint indices of the current vertex
	mat4 skin = 
		joint_weights.x * joints[int(joint_indices.x) + joint_offset] +
		joint_weights.y * joints[int(joint_indices.y) + joint_offset] +
		joint_weights.z * joints[int(joint_indices.z) + joint_offset] +
		joint_weights.w * joints[int(joint_indices.w) + joint_offset];

    gl_Position = projection_view * transform * skin * vec4(position, 1.0);
}
