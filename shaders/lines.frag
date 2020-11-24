#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in vec3 in_colour;
layout(location = 2) in flat int mode;

layout(location = 0) out vec4 out_colour;

layout(set = 0, binding = 1) uniform sampler u_sampler;
layout(set = 1, binding = 0) uniform texture2D u_texture;

void main() {
    switch (mode) {
        case 0: 
            out_colour = vec4(in_colour, 1.0);
            break;
        case 1:
            out_colour = texture(sampler2D(u_texture, u_sampler), uv);
            break;
        case 2:
            vec4 sampled = texture(sampler2D(u_texture, u_sampler), uv);
            float greyscale = (sampled.r + sampled.g + sampled.b) / 3.0;
            out_colour = vec4(vec3(greyscale), sampled.a);
            break;
    }
}
