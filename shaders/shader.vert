#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(location = 0) out vec2 out_uv;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 perspective;
    mat4 view;
};

void main() {
    out_uv = uv;

    gl_Position = perspective * view * vec4(position, 1.0);
}
