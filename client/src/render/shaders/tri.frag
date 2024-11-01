#version 300 es

precision highp float;
precision highp int;

layout(location = 0) out vec4 fragColor;

void main() {
    fragColor = vec4(1.0, 0.5, 0.2, 1.0);
}