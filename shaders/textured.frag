#version 450

layout(location = 0) in vec2 uv;
layout(location = 2) in float brightness;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 3) uniform sampler u_sampler;
layout(set = 1, binding = 0) uniform texture2D u_texture;

void main() {
    vec4 sampled = texture(sampler2D(u_texture, u_sampler), uv);
    colour = vec4(sampled.rgb * (brightness + 0.5), sampled.a);
}
