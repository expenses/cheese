#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(location = 3) in vec4 flat_colour;
layout(location = 4) in mat4 transform;

layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec4 out_flat_colour;
layout(location = 2) out vec3 out_normal;
layout(location = 3) out vec4 out_light_space;

layout(set = 0, binding = 0) uniform Perspective {
    mat4 perspective;
};

layout(set = 0, binding = 1) uniform View {
    mat4 view;
};

layout(set = 2, binding = 0) uniform ShadowUniforms {
    mat4 light_projection_view;   
};

void main() {
    out_uv = uv;
    out_flat_colour = flat_colour;
    out_normal = mat3(transpose(inverse(transform))) * normal;
    out_light_space = light_projection_view * transform * vec4(position, 1.0);

    gl_Position = perspective * view * transform * vec4(position, 1.0);
}
