#version 450

layout(location = 0) in vec2 uv;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 0) uniform texture2D u_texture;
layout(set = 0, binding = 1) uniform sampler u_sampler;

void main() {
    vec4 sampled = texture(sampler2D(u_texture, u_sampler), uv);
    float dist = length(uv - 0.5);
    colour = vec4((1.0 - dist) * sampled.xyz, 1.0);
}
