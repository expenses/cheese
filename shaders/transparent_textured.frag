#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in vec4 flat_colour;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 3) uniform sampler u_sampler;
layout(set = 1, binding = 0) uniform texture2D u_texture;

void main() {
    float alpha = texture(sampler2D(u_texture, u_sampler), uv).a;
    colour = vec4(flat_colour.rgb, flat_colour.a * alpha);
}
