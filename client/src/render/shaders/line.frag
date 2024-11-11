#version 300 es

precision highp float;
precision highp int;

layout(location = 0) out vec4 fragColor;

flat in vec4 colorInterpolant;

void main() {
    fragColor = colorInterpolant;
}
