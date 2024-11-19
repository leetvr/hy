#version 300 es

precision highp float;

const float INV_TOTAL_SAMPLES_FACTOR = 1.0 / 16.0;
uniform sampler2D u_ssaoTexture;
    
in vec2 texCoords;
    
out float FragColor;
    
void main() {
    vec2 texelSize = 1.0 / vec2(textureSize(u_ssaoTexture, 0));
    float blurred_visibility_factor = 0.0f;

    for (int t = -2; t < 2; ++t) {
        for (int s = -2; s < 2; ++s) {
            vec2 offset = vec2(float(s), float(t)) * texelSize;

            blurred_visibility_factor += texture(u_ssaoTexture, texCoords + offset).r;
        }
    }
    
    FragColor = blurred_visibility_factor * INV_TOTAL_SAMPLES_FACTOR;
}