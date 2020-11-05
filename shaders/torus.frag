#version 450

layout(location = 0) in vec3 in_colour;

layout(location = 0) out vec4 colour;

void main() {
    colour = vec4(in_colour, 1.0);
}
