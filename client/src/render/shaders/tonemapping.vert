#version 300 es

precision highp float;
precision highp int;

out vec2 clipPosition;

// Standard fullscreen triangle
void main() {
    int id = gl_VertexID;
    vec2 clip_position = vec2(float(id / 2) * 4.0 - 1.0, float(id % 2) * 4.0 - 1.0);

    gl_Position = vec4(clip_position, 1.0, 1.0);
    clipPosition = clip_position;
}