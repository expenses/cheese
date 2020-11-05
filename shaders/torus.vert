#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(location = 3) in vec3 center;
layout(location = 4) in vec3 colour;
layout(location = 5) in float radius;

layout(location = 0) out vec3 out_colour;

layout(set = 0, binding = 0) uniform Perspective {
    mat4 perspective;
};

layout(set = 0, binding = 1) uniform View {
    mat4 view;
};

mat4 mat4_from_translation(vec3 translation) {
    return mat4(
        vec4(1, 0, 0, 0),
        vec4(0, 1, 0, 0),
        vec4(0, 0, 1, 0),
        vec4(translation, 1)
    );
}

void main() {
    out_colour = colour;

    // Remove the y coordinate because we don't want to scale on that axis.
    vec3 position_no_y = vec3(position.x, 0.0, position.z);
    // Calculate what the new distance from the center should be.
    // When the radius is 1 there should be no change.
    float new_length = length(position_no_y) + radius - 1;
    // Rescale the position by the new length and add the y coordinate back.
    vec3 new = normalize(position_no_y) * new_length;
    new.y = position.y;
    
    mat4 transform = mat4_from_translation(center);
    mat4 modelview = view * transform;
    gl_Position = perspective * modelview * vec4(new, 1.0);
}
