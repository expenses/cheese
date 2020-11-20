#version 450

layout(location = 0) in vec2 uv;
layout(location = 2) in float brightness;
layout(location = 3) in vec4 light_space;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 3) uniform sampler u_sampler;
layout(set = 0, binding = 4) uniform sampler u_depth_sampler;
layout(set = 0, binding = 5) uniform texture2D shadow_map;

layout(set = 1, binding = 0) uniform texture2D u_texture;

float shadow_calculation() {
    // perform perspective divide
    vec3 projCoords = light_space.xyz / light_space.w;
    vec2 uv = vec2(
        (projCoords.x + 1.0) / 2.0,
        (1.0 - projCoords.y) / 2.0
    );

    // Objects outside the shadow map shouldn't be in shadow.
    // A better way to do this is to set a border colour for the depth texture of 1.0
    // And clamp the edges to this value, but we can't do that in wgpu, probably because border
    // colours are poorly supported in metal.
    if (uv.x > 1.0 || uv.y > 1.0 || uv.x < 0.0 || uv.y < 0.0) {
        return 0.0;
    }

    // transform to [0,1] range
    //projCoords.xy = projCoords.xy * 0.5 + 0.5;
    // get closest depth value from light's perspective (using [0,1] range fragPosLight as coords)
    float closestDepth = texture(sampler2D(shadow_map, u_sampler), uv).r;
    // get depth of current fragment from light's perspective
    float currentDepth = projCoords.z;
    // check whether current frag pos is in shadow
    float bias = 0.005;
    float shadow = currentDepth - bias > closestDepth  ? 1.0 : 0.0;

    return shadow;
}

void main() {
    vec4 sampled = texture(sampler2D(u_texture, u_sampler), uv);
    colour = vec4(sampled.rgb * (brightness + 0.5), sampled.a);
    colour.rgb = colour.rgb * (1.0 - (shadow_calculation() * 0.75));
}
