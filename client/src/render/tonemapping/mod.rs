use std::io;

use glow::HasContext;

use crate::render::compile_shaders;

const TONEMAPPING_LUT: &[u8] = include_bytes!("tony_mc_mapface.dds");

pub struct TonemappingRenderer {
    program: glow::Program,

    hdr_tex_uniform_location: Option<glow::UniformLocation>,
    lut_tex_uniform_location: Option<glow::UniformLocation>,

    lut_tex: glow::Texture,
}

impl TonemappingRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            let program = compile_shaders(
                gl,
                include_str!("../shaders/tonemapping.vert"),
                include_str!("../shaders/tonemapping.frag"),
            );

            let hdr_tex_uniform_location = gl.get_uniform_location(program, "hdrTex");
            let lut_tex_uniform_location = gl.get_uniform_location(program, "lutTex");

            gl.use_program(Some(program));

            gl.uniform_1_i32(hdr_tex_uniform_location.as_ref(), 0);
            gl.uniform_1_i32(lut_tex_uniform_location.as_ref(), 1);

            let ddsface = ddsfile::Dds::read(io::Cursor::new(&TONEMAPPING_LUT)).unwrap();
            let headerface = ddsface.header;
            let headerface10 = ddsface.header10.unwrap();

            assert_eq!(headerface.width, 48);
            assert_eq!(headerface.height, 48);
            assert_eq!(headerface.depth, Some(48));
            assert_eq!(
                headerface10.dxgi_format,
                ddsfile::DxgiFormat::R9G9B9E5_SharedExp
            );
            assert_eq!(
                headerface10.resource_dimension,
                ddsfile::D3D10ResourceDimension::Texture3D
            );

            let lut_tex = gl.create_texture().expect("Failed to create texture");
            gl.bind_texture(glow::TEXTURE_3D, Some(lut_tex));

            // Due to a cheeky little bug in glow, https://github.com/grovesNL/glow/issues/327
            // we need to first copy to an overly aligned buffer, then copy to the texture.
            // bytemuck will always zero-pad the last element if need be.
            //
            // u32 because that's what glow casts to when we use UNSIGNED_INT_5_9_9_9_REV.
            let data: Vec<u32> = bytemuck::allocation::pod_collect_to_vec(&ddsface.data);

            assert_eq!(data.len(), 48 * 48 * 48);

            gl.tex_image_3d(
                glow::TEXTURE_3D,
                0,
                glow::RGB9_E5 as _,
                48,
                48,
                48,
                0,
                glow::RGB,
                glow::UNSIGNED_INT_5_9_9_9_REV,
                Some(bytemuck::cast_slice(&data)),
            );

            gl.tex_parameter_i32(
                glow::TEXTURE_3D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as _,
            );

            gl.tex_parameter_i32(
                glow::TEXTURE_3D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _,
            );

            for wrap_target in [
                glow::TEXTURE_WRAP_S,
                glow::TEXTURE_WRAP_T,
                glow::TEXTURE_WRAP_R,
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_3D, wrap_target, glow::CLAMP_TO_EDGE as _);
            }

            Self {
                program,
                hdr_tex_uniform_location,
                lut_tex_uniform_location,
                lut_tex,
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, hdr_tex: glow::Texture) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.disable(glow::CULL_FACE);

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(hdr_tex));

            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_3D, Some(self.lut_tex));

            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.enable(glow::CULL_FACE);
        }
    }
}
