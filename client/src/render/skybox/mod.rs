use glow::HasContext;

use crate::render::compile_shaders;

const RAW_SKYBOX: &[u8] = include_bytes!("kloofendal_43d_clear_puresky_4k_cubemap.ktx2");

pub struct SkyboxRenderer {
    program: glow::Program,
    world_from_clip_uniform_location: Option<glow::UniformLocation>,

    texture: glow::Texture,
}

impl SkyboxRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            let program = compile_shaders(
                gl,
                include_str!("../shaders/skybox.vert"),
                include_str!("../shaders/skybox.frag"),
            );

            let tex_uniform_location = gl.get_uniform_location(program, "tex");
            let world_from_clip_uniform_location =
                gl.get_uniform_location(program, "worldFromClip");

            gl.use_program(Some(program));

            gl.uniform_1_i32(tex_uniform_location.as_ref(), 0);

            let tex_data = ktx2::Reader::new(RAW_SKYBOX).unwrap();
            let header = tex_data.header();

            assert_eq!(header.pixel_width, 1024);
            assert_eq!(header.pixel_height, 1024);
            assert_eq!(header.face_count, 6);
            assert_eq!(header.level_count, 10);
            assert_eq!(header.format, Some(ktx2::Format::R16G16B16A16_SFLOAT));

            let texture = gl.create_texture().expect("Failed to create texture");
            gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(texture));

            for (level_idx, level) in tex_data.levels().enumerate() {
                let mip_level_width = 1024 >> level_idx;

                let bytes_per_face = mip_level_width * mip_level_width * 4 * 2;

                assert_eq!(level.len(), bytes_per_face * 6);

                for (face_idx, face) in level.chunks_exact(bytes_per_face).enumerate() {
                    // Due to a cheeky little bug in glow, https://github.com/grovesNL/glow/issues/327
                    // we need to first copy to an overly aligned buffer, then copy to the texture.
                    // bytemuck will always zero-pad the last element if need be.
                    //
                    // u16 because that's what glow casts to when we use HALF_FLOAT.
                    let aligned_face: Vec<u16> = bytemuck::allocation::pod_collect_to_vec(&face);

                    tracing::info!(
                        "Loading level {} size {} face {} with {} bytes",
                        level_idx,
                        mip_level_width,
                        face_idx,
                        face.len()
                    );
                    gl.tex_image_2d(
                        glow::TEXTURE_CUBE_MAP_POSITIVE_X + face_idx as u32,
                        level_idx as _,
                        glow::RGBA16F as i32,
                        mip_level_width as _,
                        mip_level_width as _,
                        0,
                        glow::RGBA,
                        glow::HALF_FLOAT,
                        Some(bytemuck::cast_slice(&aligned_face)),
                    );
                }
            }

            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_MAX_LEVEL,
                header.level_count as i32 - 1,
            );

            Self {
                program,
                world_from_clip_uniform_location,
                texture,
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, origin_world_from_clip: glam::Mat4) {
        unsafe {
            gl.use_program(Some(self.program));

            gl.depth_mask(false);
            gl.disable(glow::CULL_FACE);

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(self.texture));

            gl.uniform_matrix_4_f32_slice(
                self.world_from_clip_uniform_location.as_ref(),
                false,
                bytemuck::cast_slice(origin_world_from_clip.as_ref()),
            );

            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.enable(glow::CULL_FACE);
        }
    }
}
