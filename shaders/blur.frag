#version 450

layout (location = 0) in vec2 uv;

layout (location = 0) out vec4 colour;

layout(set = 0, binding = 0) uniform sampler u_sampler;
layout(set = 0, binding = 1) uniform texture2D u_texture;
layout(set = 0, binding = 2) uniform BlurSettings {
    float blur_scale;
    float blur_strength;
    int blur_direction;
};

void main() {
	float weight[5];
	weight[0] = 0.227027;
	weight[1] = 0.1945946;
	weight[2] = 0.1216216;
	weight[3] = 0.054054;
	weight[4] = 0.016216;

 	// Get size of single texel
	vec2 tex_offset = 1.0 / textureSize(sampler2D(u_texture, u_sampler), 0) * blur_scale;
	// Current fragment's contribution
	vec3 result = texture(sampler2D(u_texture, u_sampler), uv).rgb * weight[0];
	for (int i = 1; i < 5; ++i) {
        float blur_weight = weight[i] * blur_strength;

		if (blur_direction == 1) {
			// Horizontal
            vec2 offset = vec2(tex_offset.x * i, 0.0);
			result += texture(sampler2D(u_texture, u_sampler), uv + offset).rgb * blur_weight;
			result += texture(sampler2D(u_texture, u_sampler), uv - offset).rgb * blur_weight;
		} else {
			// Vertical
            vec2 offset = vec2(0.0, tex_offset.y * i);
			result += texture(sampler2D(u_texture, u_sampler), uv + offset).rgb * blur_weight;
			result += texture(sampler2D(u_texture, u_sampler), uv - offset).rgb * blur_weight;
		}
	}
	colour = vec4(result, 1.0);
}
