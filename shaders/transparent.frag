#version 450

layout(location = 1) in vec4 flat_colour;

layout(location = 0) out vec4 colour;

void main() {
    colour = flat_colour;
}
