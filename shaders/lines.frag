#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in flat int textured;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 1) uniform sampler u_sampler;
layout(set = 1, binding = 0) uniform texture2D u_texture;

void main() {
    if (textured == 1) {
        colour = texture(sampler2D(u_texture, u_sampler), uv);
    } else {
        colour = vec4(1.0);
    }
}
