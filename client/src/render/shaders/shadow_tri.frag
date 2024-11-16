#version 300 es

precision highp float;
precision highp int;

in vec2 uvInterpolant;

uniform sampler2D tex;
uniform vec4 tint;

uniform float depthCutoff;

void main() {
    vec4 color = texture(tex, uvInterpolant);

    if (depthCutoff == 0.0) {
        return;
    }

    if (color.a < depthCutoff) {
        discard;
    }
}
