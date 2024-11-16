#version 300 es

precision highp float;
precision highp int;

in vec2 clipPosition;

layout(location = 0) out vec4 fragColor;

uniform samplerCube tex;
uniform mat4 worldFromClip;

void main() {
    vec4 clip = vec4(clipPosition, -1.0, 1.0);
    vec4 world_undiv = worldFromClip * clip;
    vec3 world = world_undiv.xyz / world_undiv.w;
    vec3 world_dir = normalize(world);

    vec3 background = texture(tex, world_dir).rgb;

    // lol basic tonemapping
    background = background / (background + vec3(1.0));

    fragColor = vec4(background, 1.0);
}
