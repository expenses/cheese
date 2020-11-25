#version 450

layout(location = 1) in vec4 flat_colour;

layout(location = 0) out vec4 colour;
layout(location = 1) out vec4 bloom;

void main() {
    colour = flat_colour;
    bloom = vec4(vec3(0.0), 1.0);
}
