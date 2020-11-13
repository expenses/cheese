#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 colour;

layout(location = 1) out vec4 out_colour;

layout(set = 0, binding = 0) uniform Perspective {
    mat4 perspective;
};

layout(set = 0, binding = 1) uniform View {
    mat4 view;
};

void main() {
    out_colour = colour;
    gl_Position = perspective * view * vec4(position, 1.0);
}
