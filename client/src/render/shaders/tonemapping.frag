#version 300 es

precision highp float;
precision highp int;
precision highp sampler2D;
precision highp sampler3D;

uniform sampler2D hdrTex;
// trilinear, clamp to edge, sampler
uniform sampler3D lutTex;

const bool USE_MCMAPFACE = false;

layout(location = 0) out vec4 fragColor;

vec3 linear_to_srgb(vec3 srgb) {
    bvec3 selector = greaterThan(srgb, vec3(0.0031308));
    vec3 under = srgb * 12.92;
    vec3 over = 1.055 * pow(srgb, vec3(1.0 / 2.4)) - 0.055;
    return mix(under, over, selector);
}

vec3 tony_mc_mapface(vec3 stimulus) {
    // Apply a non-linear transform that the LUT is encoded with.
    vec3 encoded = stimulus / (stimulus + 1.0);

    // Align the encoded range to texel centers.
    const float LUT_DIMS = 48.0;
    vec3 uv = encoded * ((LUT_DIMS - 1.0) / LUT_DIMS) + 0.5 / LUT_DIMS;

    // Note: for OpenGL, do `uv.y = 1.0 - uv.y`
    uv.y = uv.y;

    return textureLod(lutTex, uv, 0.0).rgb;
}

void main() {
    vec3 hdr_value = texelFetch(hdrTex, ivec2(round(gl_FragCoord.xy - 0.5)), 0).rgb;

    vec3 sdr_value;

    if (USE_MCMAPFACE) {
        sdr_value = tony_mc_mapface(hdr_value);
    } else {
        sdr_value = hdr_value;
    }

    fragColor = vec4(linear_to_srgb(sdr_value), 1.0);
}