#version 300 es

precision highp float;
precision highp int;
precision highp sampler2D;
precision highp sampler2DShadow;

const float LIGHT_INTENSITY = 1.5;
const float AMBIENT_INTENSITY = 0.2;

in vec3 normalInterpolant;
in vec2 uvInterpolant;
in vec3 shadowSpaceCoords;

layout(location = 0) out vec4 fragColor;

uniform sampler2D tex;
uniform sampler2DShadow shadowMap; 
uniform vec4 tint;

uniform float depthCutoff;
uniform vec3 lightDir;

float saturate(float val) {
    return clamp(val, 0.0, 1.0);
}

void main() {
    vec4 tex = texture(tex, uvInterpolant) * tint;
    fragColor = tex * LIGHT_INTENSITY;

    float nol = saturate(dot(normalInterpolant, lightDir));
    
    float shadowSize = float(textureSize(shadowMap, 0).x);
    float shadowPixelSize = 1.0 / shadowSize;
    float halfShadowPixelSize = shadowPixelSize * 0.5;

    float shadow = texture(shadowMap, shadowSpaceCoords);
    if ((shadowSpaceCoords.x < halfShadowPixelSize || shadowSpaceCoords.x > 1.0 - halfShadowPixelSize ||
         shadowSpaceCoords.y < halfShadowPixelSize || shadowSpaceCoords.y > 1.0 - halfShadowPixelSize)) {
        // Out of shadow range
        shadow = 1.0;
    }

    fragColor.rgb *= shadow;
    fragColor.rgb *= nol;

    fragColor.rgb += AMBIENT_INTENSITY * tex.rgb;

    if (depthCutoff == 0.0) {
        return;
    }

    if (fragColor.a < depthCutoff) {
        discard;
    }
}
