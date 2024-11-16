#version 300 es

precision highp float;
precision highp int;

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

out vec3 worldSpaceInterpolant;
out vec3 normalInterpolant;
out vec2 uvInterpolant;
out vec3 shadowSpaceCoords;

uniform mat4 worldFromLocal;
uniform mat4 matrix;
uniform mat4 shadowMatrix;

void main() {
    gl_Position = matrix * vec4(position, 1.0);
    worldSpaceInterpolant = (worldFromLocal * vec4(position, 1.0)).xyz;
    normalInterpolant = (worldFromLocal * vec4(normal, 0.0)).xyz;
    shadowSpaceCoords = (shadowMatrix * vec4(position, 1.0)).xyz;
    shadowSpaceCoords = shadowSpaceCoords * 0.5 + 0.5;
    uvInterpolant = uv;
}