use glam::UVec2;
use glow::HasContext;

use super::compile_shaders;

pub struct GridRenderer {
    program: glow::Program,
    matrix_location: Option<glow::UniformLocation>,
    grid_size_location: Option<glow::UniformLocation>,

    texture: super::Texture,
}

impl GridRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        let program = compile_shaders(
            gl,
            include_str!("shaders/grid.vert"),
            include_str!("shaders/grid.frag"),
        );

        unsafe {
            let matrix_location = gl.get_uniform_location(program, "clipFromWorld");
            let texture_location = gl.get_uniform_location(program, "tex");
            let grid_size_location = gl.get_uniform_location(program, "gridSize");

            gl.use_program(Some(program));
            gl.uniform_1_i32(texture_location.as_ref(), 0);

            let texture = super::Texture::new(
                gl,
                &build_grid_texture(),
                32,
                32,
                super::Filtering::Anisotropic,
                super::WrapMode::Repeat,
            );

            Self {
                program,
                matrix_location,
                grid_size_location,

                texture,
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, clip_from_world: glam::Mat4, grid_size: UVec2) {
        unsafe {
            gl.enable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ZERO,
            );

            gl.use_program(Some(self.program));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture.id));

            gl.uniform_matrix_4_f32_slice(
                self.matrix_location.as_ref(),
                false,
                clip_from_world.as_ref(),
            );
            gl.uniform_2_u32(self.grid_size_location.as_ref(), grid_size.x, grid_size.y);

            gl.draw_arrays(glow::TRIANGLES, 0, 6);

            gl.disable(glow::BLEND);
            gl.enable(glow::CULL_FACE);
        }
    }
}

fn build_grid_texture() -> Vec<u8> {
    let mut data = vec![0; 32 * 32 * 4];

    for y in 0..32 {
        for x in 0..32 {
            let idx = (y * 32 + x) * 4;
            let factor = if x == 0 || y == 0 || x == 31 || y == 31 {
                255
            } else {
                0
            };

            data[idx] = factor;
            data[idx + 1] = factor;
            data[idx + 2] = factor;
            data[idx + 3] = factor;
        }
    }

    data
}
