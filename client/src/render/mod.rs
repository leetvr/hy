mod cube_vao;
mod grid_renderer;
mod vertex;

// Re-exports
pub use cube_vao::CubeVao;
pub use vertex::Vertex;

use {
    blocks::BlockPos,
    glam::Quat,
    std::{mem, slice},
};

use bytemuck::offset_of;
use glam::{Mat4, UVec2, Vec3};
use glow::HasContext;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

use crate::{gltf::GLTFPrimitive, transform::Transform};

const POSITION_ATTRIBUTE: u32 = 0;
const NORMAL_ATTRIBUTE: u32 = 1;
const UV_ATTRIBUTE: u32 = 2;

pub struct Renderer {
    gl: glow::Context,
    canvas: HtmlCanvasElement,

    program: glow::Program,
    matrix_location: Option<glow::UniformLocation>,
    tint_location: Option<glow::UniformLocation>,

    pub camera: Camera,
    resolution: UVec2,

    grid_renderer: grid_renderer::GridRenderer,
}

impl Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        // Get the WebGL2 rendering context
        let webgl2_context = canvas
            .get_context("webgl2")?
            .ok_or("WebGL2 not supported")?
            .dyn_into::<WebGl2RenderingContext>()?;

        // Initialize glow with the WebGL2 context
        let gl = get_gl_context(webgl2_context);

        let vertex_shader_source = include_str!("shaders/tri.vert");
        let fragment_shader_source = include_str!("shaders/tri.frag");

        let program = compile_shaders(&gl, vertex_shader_source, fragment_shader_source);
        let matrix_location = unsafe { gl.get_uniform_location(program, "matrix") };
        let texture_location = unsafe { gl.get_uniform_location(program, "tex") };
        let tint_location = unsafe { gl.get_uniform_location(program, "tint") };

        unsafe { gl.uniform_1_i32(texture_location.as_ref(), 0) };

        let camera = Camera::default();

        let grid_renderer = grid_renderer::GridRenderer::new(&gl);

        Ok(Self {
            gl,
            canvas,
            program,
            matrix_location,
            tint_location,
            camera,
            resolution: UVec2::new(640, 480),
            grid_renderer,
        })
    }

    pub fn resize(&mut self, dimension: UVec2) {
        self.resolution = dimension;
    }

    pub fn render(&self, draw_calls: &[DrawCall]) {
        let gl = &self.gl;
        let aspect_ratio = self.canvas.client_width() as f32 / self.canvas.client_height() as f32;

        unsafe {
            self.gl
                .viewport(0, 0, self.resolution.x as i32, self.resolution.y as i32);

            gl.enable(glow::DEPTH_TEST);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);

            gl.depth_func(glow::LEQUAL);

            // Set the clear color
            gl.clear_color(0.1, 0.1, 0.1, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            let projection_matrix =
                Mat4::perspective_rh_gl(45.0_f32.to_radians(), aspect_ratio, 0.1, 100.0);

            let view_matrix = self.camera.view_matrix();

            gl.use_program(Some(self.program));

            for draw_call in draw_calls {
                let mvp_matrix = projection_matrix * view_matrix * draw_call.transform;

                // Set matrix
                gl.uniform_matrix_4_f32_slice(
                    self.matrix_location.as_ref(),
                    false,
                    bytemuck::cast_slice(slice::from_ref(&mvp_matrix)),
                );

                // Set tex
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(
                    glow::TEXTURE_2D,
                    Some(draw_call.primitive.diffuse_texture.id),
                );

                // Set tint
                let tint = draw_call.tint.unwrap_or(glam::Vec4::ONE);
                gl.uniform_4_f32(self.tint_location.as_ref(), tint.x, tint.y, tint.z, tint.w);

                gl.bind_vertex_array(Some(draw_call.primitive.vao));

                gl.draw_elements(
                    glow::TRIANGLES,
                    draw_call.primitive.index_count as i32,
                    glow::UNSIGNED_INT,
                    draw_call.primitive.index_start as i32 * mem::size_of::<u32>() as i32,
                );
            }

            self.grid_renderer.render(
                &self.gl,
                projection_matrix * view_matrix,
                UVec2::new(64, 64),
            );

            gl.flush();
        }
    }

    pub fn create_cube_vao(&self) -> CubeVao {
        CubeVao::new(&self.gl)
    }

    pub fn create_texture_from_image(&self, data: &[u8], width: u32, height: u32) -> Texture {
        Texture::new(
            &self.gl,
            data,
            width,
            height,
            Filtering::Nearest,
            WrapMode::Clamp,
        )
    }
}

fn compile_shaders(
    gl: &glow::Context,
    vertex_shader_source: &str,
    fragment_shader_source: &str,
) -> glow::Program {
    unsafe {
        // Compile the vertex shader
        let vertex_shader = gl
            .create_shader(glow::VERTEX_SHADER)
            .expect("Cannot create shader");
        gl.shader_source(vertex_shader, vertex_shader_source);
        gl.compile_shader(vertex_shader);

        // Check for compilation errors
        if !gl.get_shader_compile_status(vertex_shader) {
            let log = gl.get_shader_info_log(vertex_shader);
            tracing::info!("Vertex shader compilation failed: {}", log);
            // return Err(JsValue::from_str(&log));
            panic!("Vertex shader compilation failed: {}", log);
        }
        // Compile the fragment shader
        let fragment_shader = gl
            .create_shader(glow::FRAGMENT_SHADER)
            .expect("Cannot create shader");
        gl.shader_source(fragment_shader, fragment_shader_source);
        gl.compile_shader(fragment_shader);

        // Check for compilation errors
        if !gl.get_shader_compile_status(fragment_shader) {
            let log = gl.get_shader_info_log(fragment_shader);
            tracing::info!("Fragment shader compilation failed: {}", log);
            // return Err(JsValue::from_str(&log));
            panic!("Fragment shader compilation failed: {}", log);
        }

        // Link the shaders into a program
        let program = gl.create_program().expect("Cannot create program");
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);

        // Check for linking errors
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            tracing::info!("Program linking failed: {}", log);
            // return Err(JsValue::from_str(&log));
            panic!("Program linking failed: {}", log);
        }

        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
        program
    }
}

#[derive(Debug, Clone)]
pub struct RenderPrimitive {
    vao: glow::VertexArray,
    diffuse_texture: Texture,
    index_start: u32,
    index_count: u32,
}

impl RenderPrimitive {
    fn from_gltf(gl: &glow::Context, primitive: &GLTFPrimitive) -> Self {
        unsafe {
            let vao = gl
                .create_vertex_array()
                .expect("Failed to create vertex array");
            gl.bind_vertex_array(Some(vao));

            let vertex_buffer = gl.create_buffer().expect("Failed to create buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&primitive.vertices),
                glow::STATIC_DRAW,
            );

            let index_buffer = gl.create_buffer().expect("Failed to create buffer");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&primitive.indices),
                glow::STATIC_DRAW,
            );

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

            let diffuse_texture =
                if let Some(ref base_color_texture) = primitive.material.base_colour_texture {
                    Texture::new(
                        gl,
                        &base_color_texture.data,
                        base_color_texture.dimensions.x,
                        base_color_texture.dimensions.y,
                        Filtering::Nearest,
                        WrapMode::Clamp,
                    )
                } else {
                    let scaled = primitive.material.base_colour_factor * 255.0;
                    let bytes = scaled.to_array().map(|x| x as u8);
                    Texture::new(gl, &bytes, 1, 1, Filtering::Nearest, WrapMode::Clamp)
                };

            Self {
                vao,
                diffuse_texture,
                index_start: 0,
                index_count: primitive.indices.len() as u32,
            }
        }
    }
}

#[derive(Debug)]
pub struct RenderMesh {
    primitives: Vec<RenderPrimitive>,
}

impl RenderMesh {
    pub fn from_gltf(renderer: &Renderer, mesh: &crate::gltf::GLTFMesh) -> Self {
        let mut primitives = Vec::new();

        for primitive in &mesh.primitives {
            primitives.push(RenderPrimitive::from_gltf(&renderer.gl, primitive));
        }

        Self { primitives }
    }
}

#[derive(Debug)]
pub struct RenderModel {
    meshes: Vec<RenderMesh>,
}

impl RenderModel {
    pub fn from_gltf(renderer: &Renderer, gltf: &crate::gltf::GLTFModel) -> Self {
        let mut meshes = Vec::new();

        for mesh in &gltf.meshes {
            meshes.push(RenderMesh::from_gltf(renderer, mesh));
        }

        Self { meshes }
    }
}

pub fn build_render_plan(
    models: &[crate::gltf::GLTFModel],
    render_model: &[RenderModel],
    transform: Transform,
) -> Vec<DrawCall> {
    let mut render_objects = Vec::new();

    for (idx, model) in models.iter().enumerate() {
        build_render_plan_recursive(
            &mut render_objects,
            model,
            &render_model[idx],
            model.root_node_idx,
            transform,
        );
    }

    render_objects
}

fn build_render_plan_recursive(
    draw_calls: &mut Vec<DrawCall>,
    gltf: &crate::gltf::GLTFModel,
    render_model: &RenderModel,
    current_node: usize,
    parent_transform: Transform,
) {
    let node = &gltf.nodes[current_node];

    let transform = parent_transform * node.current_transform;

    if let Some(mesh) = node.mesh {
        let render_mesh = &render_model.meshes[mesh];
        for primitive in &render_mesh.primitives {
            draw_calls.push(DrawCall {
                primitive: primitive.clone(),
                transform: transform.as_affine().into(),
                tint: None,
            });
        }
    }

    for &child in &node.children {
        build_render_plan_recursive(draw_calls, gltf, render_model, child, transform);
    }
}

#[derive(Debug)]
pub struct DrawCall {
    pub primitive: RenderPrimitive,
    pub transform: glam::Mat4,
    pub tint: Option<glam::Vec4>,
}

// This is the only thing keeping us from building this crate on non-wasm32 targets
// This function just hides build errors on non-wasm32 targets so we can use rust-analyzer
fn get_gl_context(_webgl2_context: WebGl2RenderingContext) -> glow::Context {
    #[cfg(not(target_arch = "wasm32"))]
    panic!("This function should only be called on wasm32 target");

    #[cfg(target_arch = "wasm32")]
    glow::Context::from_webgl2_context(_webgl2_context)
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, -5.0),
            rotation: Default::default(),
        }
    }
}

impl Camera {
    pub fn view_matrix(&self) -> Mat4 {
        (Mat4::from_translation(self.position) * Mat4::from_quat(self.rotation)).inverse()
    }
}

pub enum Filtering {
    Nearest,
    Anisotropic,
}

pub enum WrapMode {
    Clamp,
    Repeat,
}

#[derive(Debug, Clone)]
pub struct Texture {
    id: glow::Texture,
}

impl Texture {
    pub fn new(
        gl: &glow::Context,
        data: &[u8],
        width: u32,
        height: u32,
        filtering: Filtering,
        wrap: WrapMode,
    ) -> Self {
        let id = unsafe { gl.create_texture().expect("Failed to create texture") };
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(id));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(data),
            );
            let wrap_raw = match wrap {
                WrapMode::Clamp => glow::CLAMP_TO_EDGE,
                WrapMode::Repeat => glow::REPEAT,
            } as i32;

            let min_filter;
            let max_filter;
            match filtering {
                Filtering::Nearest => {
                    min_filter = glow::NEAREST;
                    max_filter = glow::NEAREST;
                }
                Filtering::Anisotropic => {
                    min_filter = glow::LINEAR_MIPMAP_LINEAR;
                    max_filter = glow::LINEAR;
                }
            }

            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap_raw);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap_raw);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                min_filter as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                max_filter as i32,
            );

            if let Filtering::Anisotropic = filtering {
                gl.tex_parameter_f32(glow::TEXTURE_2D, glow::TEXTURE_MAX_ANISOTROPY, 16.0);
                gl.generate_mipmap(glow::TEXTURE_2D);
            }
        }

        Self { id }
    }
}

pub fn build_cube_draw_calls<'a>(
    vao: &CubeVao,
    blocks: impl Iterator<Item = (BlockPos, &'a [Texture; 6])>,
    tint: Option<glam::Vec4>,
) -> Vec<DrawCall> {
    let mut draw_calls = Vec::new();

    for (pos, textures) in blocks {
        let transform = Transform::new_with_scale(pos, glam::Quat::IDENTITY, glam::Vec3::ONE);

        // One texture for each face
        for (i, texture) in textures.iter().enumerate() {
            let base_index = i * 6;
            let draw_call = DrawCall {
                primitive: RenderPrimitive {
                    vao: vao.vao,
                    diffuse_texture: (*texture).clone(),
                    index_start: base_index as u32,
                    index_count: 6,
                },
                transform: transform.as_affine().into(),
                tint,
            };

            draw_calls.push(draw_call);
        }
    }

    draw_calls
}
