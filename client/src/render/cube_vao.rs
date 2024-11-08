use std::mem::{self, offset_of};

use glow::HasContext;

use super::{Vertex, NORMAL_ATTRIBUTE, POSITION_ATTRIBUTE, UV_ATTRIBUTE};

pub struct CubeVao {
    pub vao: glow::VertexArray,
    _vertex_buffer: glow::Buffer,
    _index_buffer: glow::Buffer,
}

impl CubeVao {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            // Start by creating an empty VAO
            let vao = gl
                .create_vertex_array()
                .expect("Failed to create vertex array");
            gl.bind_vertex_array(Some(vao));

            let vertices: [Vertex; 24] = [
                // North face (z = 1.0, normal (0, 0, 1))
                Vertex {
                    position: [0.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [0.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 1.0],
                },
                // South face (z = 0.0, normal (0, 0, -1))
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                    uv: [0.0, 1.0],
                },
                // East face (x = 0.0, normal (-1, 0, 0))
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [-1.0, 0.0, 0.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [0.0, 0.0, 1.0],
                    normal: [-1.0, 0.0, 0.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [0.0, 1.0, 1.0],
                    normal: [-1.0, 0.0, 0.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [-1.0, 0.0, 0.0],
                    uv: [0.0, 1.0],
                },
                // West face (x = 1.0, normal (1, 0, 0))
                Vertex {
                    position: [1.0, 0.0, 1.0],
                    normal: [1.0, 0.0, 0.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [1.0, 0.0, 0.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [1.0, 0.0, 0.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [1.0, 0.0, 0.0],
                    uv: [0.0, 1.0],
                },
                // Top face (y = 1.0, normal (0, 1, 0))
                Vertex {
                    position: [0.0, 1.0, 1.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 1.0],
                },
                // Bottom face (y = 0.0, normal (0, -1, 0))
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, -1.0, 0.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, -1.0, 0.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 1.0],
                    normal: [0.0, -1.0, 0.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [0.0, 0.0, 1.0],
                    normal: [0.0, -1.0, 0.0],
                    uv: [0.0, 1.0],
                },
            ];

            // Now stash some vertices into it
            let vertex_buffer = gl.create_buffer().expect("Failed to create buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            // Now stash some indices into it

            #[rustfmt::skip]
            let indices: [u32; 36] = [
                // Front face
                0, 1, 2, 2, 3, 0,
                // Back face
                4, 5, 6, 6, 7, 4,
                // Left face
                8, 9, 10, 10, 11, 8,
                // Right face
                12, 13, 14, 14, 15, 12,
                // Top face
                16, 17, 18, 18, 19, 16,
                // Bottom face
                20, 21, 22, 22, 23, 20,
            ];

            let index_buffer = gl.create_buffer().expect("Failed to create buffer");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                glow::STATIC_DRAW,
            );

            // Now let's set up some vertex attributes

            let stride = mem::size_of::<Vertex>() as i32;

            gl.enable_vertex_attrib_array(POSITION_ATTRIBUTE);
            gl.vertex_attrib_pointer_f32(
                POSITION_ATTRIBUTE,
                3,
                glow::FLOAT,
                false,
                stride,
                offset_of!(Vertex, position) as i32,
            );

            gl.enable_vertex_attrib_array(NORMAL_ATTRIBUTE);
            gl.vertex_attrib_pointer_f32(
                NORMAL_ATTRIBUTE,
                3,
                glow::FLOAT,
                false,
                stride,
                offset_of!(Vertex, normal) as i32,
            );

            gl.enable_vertex_attrib_array(UV_ATTRIBUTE);
            gl.vertex_attrib_pointer_f32(
                UV_ATTRIBUTE,
                2,
                glow::FLOAT,
                false,
                stride,
                offset_of!(Vertex, uv) as i32,
            );

            gl.bind_vertex_array(None);

            Self {
                vao,
                _vertex_buffer: vertex_buffer,
                _index_buffer: index_buffer,
            }
        }
    }
}
