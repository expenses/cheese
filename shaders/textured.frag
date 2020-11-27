#version 450

layout(location = 0) in vec2 uv;
layout(location = 2) in vec3 normal;
layout(location = 3) in vec4 light_space;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 2) uniform Sun {
    vec3 sun_direction;
};

layout(set = 0, binding = 3) uniform sampler u_sampler;
layout(set = 0, binding = 4) uniform sampler u_depth_sampler;
layout(set = 0, binding = 5) uniform texture2D shadow_map;

layout(set = 1, binding = 0) uniform texture2D u_texture;

float shadow_calculation() {
    // Divide by perspective
    vec3 coords = light_space.xyz / light_space.w;
    vec2 uv = vec2(
        (coords.x + 1.0) / 2.0,
        (1.0 - coords.y) / 2.0
    );

    // Objects outside the shadow map shouldn't be in shadow.
    // A better way to do this is to set a border colour for the depth texture of 1.0
    // And clamp the edges to this value, but we can't do that in wgpu 0.6, only on the git master.
    if (uv.x > 1.0 || uv.y > 1.0 || uv.x < 0.0 || uv.y < 0.0) {
        return 0.0;
    }

    float current_depth = coords.z;
    float max_bias = 0.01;
    float min_bias = 0.001;
    float bias = max(max_bias * (1.0 - dot(normalize(normal), normalize(sun_direction))), min_bias);

    float shadow = 0.0;
    float texel_size = 1.0 / textureSize(sampler2D(shadow_map, u_depth_sampler), 0);
    for (int x =  -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            float depth = texture(sampler2D(shadow_map, u_depth_sampler), uv + vec2(x, y) * texel_size).r;
            shadow += current_depth - bias > depth ? 1.0 : 0.0;
        }
    }

    shadow /= 9.0;

    return shadow;
}

void main() {
    float brightness = max(0.0, dot(normalize(normal), normalize(sun_direction)));

    vec4 sampled = texture(sampler2D(u_texture, u_sampler), uv);
    colour = vec4(sampled.rgb * (brightness + 0.5), sampled.a);
    colour.rgb = colour.rgb * (1.0 - (shadow_calculation() * 0.75));
}
