#version 300 es

precision highp float;

uniform sampler2D ssaoBlurTexture;

in vec2 texCoords;

out vec4 fragColor;

void main() {
    vec4 color = textureLod(ssaoBlurTexture, texCoords, 0.0);
    fragColor = vec4(0.0, 0.0, 0.0, 1.0 - color.r);
}