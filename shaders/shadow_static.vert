#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(location = 3) in vec4 flat_colour;
layout(location = 4) in mat4 transform;

layout(set = 0, binding = 0) uniform Light {
    mat4 projection_view;
};

void main() {
    gl_Position = projection_view * transform * vec4(position, 1.0);
}
