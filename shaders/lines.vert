#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec3 colour;
layout(location = 3) in int textured;

layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec3 out_colour;
layout(location = 2) out flat int out_textured;

layout(set = 0, binding = 0) uniform Uniforms {
    vec2 screen_dimensions;
};

void main() {
    out_uv = uv;
    out_colour = colour;
    out_textured = textured;

    vec2 adjusted_position = vec2(
        (position.x / screen_dimensions.x * 2.0) - 1.0,
        1.0 - (position.y / screen_dimensions.y * 2.0)
    );
    gl_Position = vec4(adjusted_position, 0.0, 1.0);
}
