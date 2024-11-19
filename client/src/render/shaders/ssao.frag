#version 300 es

precision highp float;

const int MAX_KERNEL_SIZE = 20;
const float INV_MAX_KERNEL_SIZE_F = 1.0/float(MAX_KERNEL_SIZE);
const vec2 HALF_2 = vec2(0.5);

uniform mat4 u_projection;
uniform mat4 u_projection_inverse;
      
uniform sampler2D u_depthMap;
uniform sampler2D u_noise;
// visibility radius
uniform float u_sampleRad;
uniform vec3 u_kernel[MAX_KERNEL_SIZE];
uniform vec2 u_noiseScale;
in vec2 texCoords;

// the fragment shader output is a single component floating point value between 0.0 to 1.0.
out float FragColor;

// this function calculates view position of a fragment from its depth value
// sampled from the depth texture.
vec3 calcViewPosition(vec2 coords) {
    float fragmentDepth = texture(u_depthMap, coords).r;
    
    // Convert coords and fragmentDepth to 
    // normalized device coordinates (clip space)
    vec4 ndc = vec4(
        coords.x * 2.0 - 1.0, 
        coords.y * 2.0 - 1.0, 
        fragmentDepth * 2.0 - 1.0, 
        1.0
    );
    
    // Transform to view space using inverse camera projection matrix.
    vec4 vs_pos = u_projection_inverse * ndc;
    
    // since we used a projection transformation (even if it was in inverse)
    // we need to convert our homogeneous coordinates using the perspective divide.
    vs_pos.xyz = vs_pos.xyz / vs_pos.w;
        
    return vs_pos.xyz;
}

void main() {
    vec3 viewPos = calcViewPosition(texCoords);

    // the dFdy and dFdX are glsl functions used to calculate two vectors in view space 
    // that lie on the plane of the surface being drawn. We pass the view space position to these functions.
    // The cross product of these two vectors give us the normal in view space.
    vec3 viewNormal = cross(dFdy(viewPos.xyz), dFdx(viewPos.xyz));

    // The normal is initilly away from the screen based on the order in which we calculate the cross products. 
    // Here, we need to invert it to point towards the screen by multiplying by -1. 
    // Then we normalize this vector to get a unit normal vector.
    viewNormal = normalize(viewNormal * -1.0);
    // we calculate a random offset using the noise texture sample. 
    //This will be applied as rotation to all samples for our current fragments.
    vec3 randomVec = texture(u_noise, texCoords * u_noiseScale).xyz; 
    // here we apply the Gramm-Schmidt process to calculate the TBN matrix 
    // with a random offset applied. 
    vec3 tangent = normalize(randomVec - viewNormal * dot(randomVec, viewNormal));
    vec3 bitangent = cross(viewNormal, tangent);
    mat3 TBN = mat3(tangent, bitangent, viewNormal); 
    float occlusion_factor = 0.0;
    for (int i = 0 ; i < MAX_KERNEL_SIZE ; i++) {
        vec3 samplePos = TBN * u_kernel[i];

        // here we calculate the sampling point position in view space.
        samplePos = viewPos + samplePos * u_sampleRad;

        // now using the sampling point offset
        vec4 offset = vec4(samplePos, 1.0);
        offset = u_projection * offset;
        offset.xy /= offset.w;
        offset.xy = offset.xy * HALF_2 + HALF_2;

        // this is the geometry's depth i.e. the view_space_geometry_depth
        // this value is negative in my coordinate system
        float geometryDepth = calcViewPosition(offset.xy).z;
        
        float rangeCheck = smoothstep(0.0, 1.0, u_sampleRad / abs(viewPos.z - geometryDepth));
        
        // samplePos.z is the sample's depth i.e. the view_space_sampling_position depth
        // this value is negative in my coordinate system
        // for occlusion to be true the geometry's depth should be greater or equal (equal or less negative and consequently closer to the camera) than the sample's depth
        occlusion_factor += float(geometryDepth >= samplePos.z + 0.0001) * rangeCheck; 
    }

    // we will divide the accmulated occlusion by the number of samples to get the average occlusion value. 
    float average_occlusion_factor = occlusion_factor * INV_MAX_KERNEL_SIZE_F;
    
    float visibility_factor = 1.0 - average_occlusion_factor;

    // We can raise the visibility factor to a power to make the transition
    // more sharp. Experiment with the value of this power to see what works best for you.
    // Even after raising visibility to a power > 1, the range still remains between [0.0, 1.0].
    visibility_factor = pow(visibility_factor, 2.0);

    FragColor = visibility_factor;
}