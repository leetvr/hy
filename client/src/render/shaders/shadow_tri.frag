#version 300 es

precision highp float;
precision highp int;

in vec2 uvInterpolant;

layout(location = 0) out vec4 fragColor;

uniform sampler2D tex;
uniform vec4 tint;

uniform float depthCutoff;

void main() {
    fragColor = texture(tex, uvInterpolant);

    if (depthCutoff == 0.0) {
        return;
    }

    if (fragColor.a < depthCutoff) {
        discard;
    }
}
