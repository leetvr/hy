use glam::{Mat4, Vec3};
use glow::HasContext;
use gltf::camera;
use nanorand::Rng;

use crate::render::{compile_shaders, Camera};

pub struct SsaoRenderer {
    ssao_program: glow::Program,
    blur_program: glow::Program,
    combine_program: glow::Program,

    raw_framebuffer: glow::Framebuffer,
    raw_texture: glow::Texture,

    blur_framebuffer: glow::Framebuffer,
    blur_texture: glow::Texture,

    kernel: Vec<Vec3>,
    noise_texture: glow::Texture,
}

impl SsaoRenderer {
    pub fn new(gl: &glow::Context, dimensions: glam::UVec2) -> Self {
        let half_dimensions = dimensions / 2;

        unsafe {
            let ssao_program = compile_shaders(
                gl,
                include_str!("shaders/ssao.vert"),
                include_str!("shaders/ssao.frag"),
            );
            let blur_program = compile_shaders(
                gl,
                include_str!("shaders/ssao.vert"),
                include_str!("shaders/ssaoBlur.frag"),
            );
            let combine_program = compile_shaders(
                gl,
                include_str!("shaders/ssao.vert"),
                include_str!("shaders/ssaoCombine.frag"),
            );

            let raw_framebuffer = gl.create_framebuffer().unwrap();
            let raw_texture = gl.create_texture().unwrap();

            let blur_framebuffer = gl.create_framebuffer().unwrap();
            let blur_texture = gl.create_texture().unwrap();

            let noise_texture = gl.create_texture().unwrap();

            gl.bind_texture(glow::TEXTURE_2D, Some(raw_texture));
            gl.tex_storage_2d(
                glow::TEXTURE_2D,
                1,
                glow::R8,
                half_dimensions.x as i32,
                half_dimensions.y as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(raw_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(raw_texture),
                0,
            );
            gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
            gl.read_buffer(glow::COLOR_ATTACHMENT0);

            gl.bind_texture(glow::TEXTURE_2D, Some(blur_texture));
            gl.tex_storage_2d(
                glow::TEXTURE_2D,
                1,
                glow::R8,
                half_dimensions.x as i32,
                half_dimensions.y as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(blur_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(blur_texture),
                0,
            );
            gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
            gl.read_buffer(glow::COLOR_ATTACHMENT0);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            let noise = generate_ssao_noise(16);
            gl.bind_texture(glow::TEXTURE_2D, Some(noise_texture));
            gl.tex_storage_2d(glow::TEXTURE_2D, 1, glow::RGB16F, 4, 4);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                4,
                4,
                glow::RGB,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(bytemuck::cast_slice(&noise)),
            );

            Self {
                ssao_program,
                blur_program,
                combine_program,

                raw_framebuffer,
                raw_texture,

                blur_framebuffer,
                blur_texture,

                kernel: generate_ssao_kernel(64),
                noise_texture,
            }
        }
    }

    pub fn resize(&mut self, gl: &glow::Context, dimensions: glam::UVec2) {
        let half_dimensions = dimensions / 2;

        unsafe {
            gl.delete_texture(self.raw_texture);
            gl.delete_texture(self.blur_texture);

            self.raw_texture = gl.create_texture().unwrap();
            self.blur_texture = gl.create_texture().unwrap();

            gl.bind_texture(glow::TEXTURE_2D, Some(self.raw_texture));
            gl.tex_storage_2d(
                glow::TEXTURE_2D,
                1,
                glow::R8,
                half_dimensions.x as i32,
                half_dimensions.y as i32,
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.raw_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(self.raw_texture),
                0,
            );

            gl.bind_texture(glow::TEXTURE_2D, Some(self.blur_texture));
            gl.tex_storage_2d(
                glow::TEXTURE_2D,
                1,
                glow::R8,
                half_dimensions.x as i32,
                half_dimensions.y as i32,
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.blur_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(self.blur_texture),
                0,
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        resolution: glam::UVec2,
        input_depth_texture: glow::Texture,
        hdr_framebuffer: glow::Framebuffer,
        clip_from_view: Mat4,
    ) {
        unsafe {
            let half_resolution = resolution / 2;
            gl.viewport(0, 0, half_resolution.x as i32, half_resolution.y as i32);

            // ssaoShaderProgram is the compiled WebGL shader program
            gl.use_program(Some(self.ssao_program));
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.ssao_program, "u_projection")
                    .as_ref(),
                false,
                clip_from_view.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.ssao_program, "u_projection_inverse")
                    .as_ref(),
                false,
                clip_from_view.inverse().as_ref(),
            );
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(input_depth_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.ssao_program, "u_depthMap")
                    .as_ref(),
                0,
            );

            gl.active_texture(glow::TEXTURE0 + 1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.noise_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.ssao_program, "u_noise")
                    .as_ref(),
                1,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.ssao_program, "u_sampleRad")
                    .as_ref(),
                // this the visibility radius in view space
                0.5,
            );
            gl.uniform_2_f32_slice(
                gl.get_uniform_location(self.ssao_program, "u_noiseScale")
                    .as_ref(),
                (resolution.as_vec2() / 4.0).as_ref(),
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.ssao_program, "u_kernel")
                    .as_ref(),
                bytemuck::cast_slice(&self.kernel),
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.raw_framebuffer));
            // here we clear the previously rendered values from the ssao raw texture
            gl.color_mask(true, true, true, true);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            // Here we draw a full screen quad using an already set up Vertex Array Object
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            // Now our SSAO raw texture is populated with occlusion factor data

            // After this we will use the SSAO raw texture as input and blur the output to
            // the SSAO blur texture. We will use the gausian blur shader for this. To account
            // for depth when blurring so that geometry edges are not blurred into other geometry
            // we can use a bi-lateral blur algorithm which i have not discussed for simplicity.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.blur_framebuffer));
            // here we clear the previously rendered values from the ssao blur texture
            gl.clear(glow::COLOR_BUFFER_BIT);
            // ssaoBlurShaderProgram is the compiled WebGL shader program for applying gausian blur
            gl.use_program(Some(self.blur_program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.raw_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.blur_program, "u_ssaoTexture")
                    .as_ref(),
                0,
            );
            // we again draw a full screen quad using the previously bound vertex array object
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            // Now our SSAO blur texture is populated with occlusion factor data
            // we can now combine the SSAO blur texture with the color texture to get the final
            // SSAO effect
            gl.viewport(0, 0, resolution.x as i32, resolution.y as i32);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(hdr_framebuffer));

            gl.enable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);

            gl.use_program(Some(self.combine_program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.blur_texture));

            gl.uniform_1_i32(
                gl.get_uniform_location(self.combine_program, "ssaoBlurTexture")
                    .as_ref(),
                0,
            );

            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.depth_mask(true);
            gl.enable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
        }
    }
}

fn random_f32_0_1() -> f32 {
    let uint: u32 = nanorand::tls_rng().generate();

    uint as f32 / u32::MAX as f32
}

fn generate_ssao_kernel(sample_count: u32) -> Vec<Vec3> {
    let mut kernel = vec![Vec3::ZERO; sample_count as usize];
    for i in 0..sample_count {
        let sample = Vec3::new(
            random_f32_0_1() * 2.0 - 1.0,
            random_f32_0_1() * 2.0 - 1.0,
            random_f32_0_1(),
        )
        .normalize_or_zero();
        // After normalization the sample points lie on the surface of the hemisphere
        // and each sample point vector has the same length.
        // We want to randomly change the sample points to sample more
        // points inside the hemisphere as close to our fragment as possible.
        // we will use an accelerating interpolation to do this.
        let scale = i as f32 / sample_count as f32;
        // you can use a standard math library to perform the lerp function or
        // write your own.
        let interpolated_scale = glam::FloatExt::lerp(0.1, 1.0, scale * scale);
        kernel[i as usize] = sample * interpolated_scale;
    }
    kernel
}

fn generate_ssao_noise(noise_size: u32) -> Vec<Vec3> {
    let mut noise_array = vec![Vec3::ZERO; noise_size as usize];
    for i in 0..noise_size {
        let noise = Vec3::new(
            random_f32_0_1() * 2.0 - 1.0,
            random_f32_0_1() * 2.0 - 1.0,
            0.0,
        );
        noise_array[i as usize] = noise;
    }
    noise_array
}
