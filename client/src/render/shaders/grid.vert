#version 300 es

precision highp float;
precision highp int;

out vec2 uvInterpolant;

const vec2 uvs[4] = vec2[4](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 1.0)
);

const int indices[6] = int[6](
    0, 1, 2,
    2, 1, 3
);

uniform mat4 clipFromWorld;
uniform uvec2 gridSize;

void main() {
    int index = indices[gl_VertexID];

    vec2 position = uvs[index] * vec2(gridSize);

    gl_Position = clipFromWorld * vec4(position.x, -0.01, position.y, 1.0);
    uvInterpolant = position;
}