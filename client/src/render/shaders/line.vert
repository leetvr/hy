#version 300 es

precision highp float;
precision highp int;

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;

flat out vec4 colorInterpolant;

uniform mat4 clipFromWorld;

void main() {
    gl_Position = clipFromWorld * vec4(position, 1.0);
    colorInterpolant = color;
}