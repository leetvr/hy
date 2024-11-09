#version 300 es

precision highp float;
precision highp int;

in vec2 uvInterpolant;

layout(location = 0) out vec4 fragColor;

uniform sampler2D tex;

void main() {
    fragColor = texture(tex, uvInterpolant);
    if (fragColor.a < 1.0) {
        discard;
    }
}
