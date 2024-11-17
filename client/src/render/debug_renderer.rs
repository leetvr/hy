use bytemuck::{Pod, Zeroable};
use glow::HasContext;

use crate::render::compile_shaders;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct DebugLine {
    // We technically only actually need one color, but that's not entirely possible to express with normal vbos.
    //
    // This is actually two vertices.
    start: [f32; 3],
    start_color: [f32; 4],
    end: [f32; 3],
    end_color: [f32; 4],
}

impl DebugLine {
    #[allow(unused)]
    pub fn new(start: glam::Vec3, end: glam::Vec3) -> Self {
        // Default to a yellow-ish orange color.
        let color = glam::Vec4::new(1.0, 0.8, 0.2, 1.0);
        Self::new_with_color(start, end, color)
    }

    #[allow(unused)]
    pub fn new_with_color(start: glam::Vec3, end: glam::Vec3, color: glam::Vec4) -> Self {
        Self {
            start: start.into(),
            start_color: color.into(),
            end: end.into(),
            end_color: color.into(),
        }
    }
}

impl From<net_types::DebugLine> for DebugLine {
    fn from(value: net_types::DebugLine) -> Self {
        let color = [1.0, 0.8, 0.2, 1.0];
        Self {
            start: value.start.to_array(),
            start_color: color,
            end: value.end.to_array(),
            end_color: color,
        }
    }
}

pub struct DebugRenderer {
    program: glow::Program,
    matrix_location: Option<glow::UniformLocation>,
}

impl DebugRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        let program = compile_shaders(
            gl,
            include_str!("shaders/line.vert"),
            include_str!("shaders/line.frag"),
        );

        unsafe {
            let matrix_location = gl.get_uniform_location(program, "clipFromWorld");

            Self {
                program,
                matrix_location,
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, clip_from_world: glam::Mat4, lines: &[DebugLine]) {
        unsafe {
            gl.use_program(Some(self.program));

            gl.uniform_matrix_4_f32_slice(
                self.matrix_location.as_ref(),
                false,
                clip_from_world.as_ref(),
            );

            let vertex_buffer_data: &[u8] = bytemuck::cast_slice(&lines);

            let vertex_array = gl.create_vertex_array().expect("Create VAO");
            gl.bind_vertex_array(Some(vertex_array));

            let vertex_buffer = gl.create_buffer().expect("Create VBO");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertex_buffer_data, glow::STATIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                std::mem::size_of::<DebugLine>() as i32 / 2,
                bytemuck::offset_of!(DebugLine, start) as i32,
            );

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                4,
                glow::FLOAT,
                false,
                std::mem::size_of::<DebugLine>() as i32 / 2,
                bytemuck::offset_of!(DebugLine, start_color) as i32,
            );

            gl.draw_arrays(glow::LINES, 0, lines.len() as i32 * 2);
        }
    }
}
