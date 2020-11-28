#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in vec4 flat_colour;

layout(location = 0) out vec4 colour;
layout(location = 1) out vec4 bloom_colour;

layout(set = 0, binding = 3) uniform sampler u_sampler;
layout(set = 1, binding = 0) uniform texture2D u_texture;

void main() {
    colour = texture(sampler2D(u_texture, u_sampler), uv);
    float intensity = flat_colour.a;
    bloom_colour = vec4(colour.rgb * intensity, 1.0);
}
