#version 300 es

precision highp float;
          
out vec2 texCoords;
          
void main() {       
    int id = gl_VertexID;
    vec2 clip_position = vec2(float(id / 2) * 4.0 - 1.0, float(id % 2) * 4.0 - 1.0);

    gl_Position = vec4(clip_position, 1.0, 1.0);
	
	// convert vertex positions from range [-1,-1] to range [0,1]
	texCoords = gl_Position.xy * 0.5 + 0.5;
}