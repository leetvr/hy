#version 300 es

precision highp float;
precision highp int;

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

out vec2 uvInterpolant;

uniform mat4 matrix;

void main() {
    gl_Position = matrix * vec4(position, 1.0);
    uvInterpolant = uv;
}