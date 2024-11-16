mod cube_vao;
mod debug_renderer;
mod grid_renderer;
mod skybox;
mod vertex;

// Re-exports
pub use cube_vao::CubeVao;
pub use debug_renderer::DebugLine;
pub use vertex::Vertex;

use {
    blocks::BlockPos,
    glam::Quat,
    std::{mem, slice},
};

use bytemuck::{offset_of, Pod, Zeroable};
use glam::{Mat4, UVec2, UVec3, Vec3};
use glow::HasContext;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

use crate::{
    gltf::{GLTFPrimitive, TransparencyType},
    transform::Transform,
};

const POSITION_ATTRIBUTE: u32 = 0;
const NORMAL_ATTRIBUTE: u32 = 1;
const UV_ATTRIBUTE: u32 = 2;

const SHADOW_SIZE: UVec2 = UVec2::splat(2048);
const LIGHT_DIRECTION: Vec3 = Vec3::new(-1.0, -1.0, -1.0);

const MAX_LIGHTS: usize = 64;

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
struct LightBuffer {
    light_count: u32,
    _padding: [u32; 3],
    lights: [Light; MAX_LIGHTS],
}

impl Default for LightBuffer {
    fn default() -> Self {
        Self {
            light_count: Default::default(),
            _padding: Default::default(),
            lights: [Default::default(); MAX_LIGHTS],
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Zeroable, Pod)]
pub struct Light {
    pub position: Vec3,
    pub distance: f32,
    pub color: Vec3,
    pub _padding: f32,
}

pub struct Renderer {
    gl: glow::Context,
    canvas: HtmlCanvasElement,

    forward_program: PrimaryProgram,
    shadow_program: PrimaryProgram,

    shadow_target: ShadowTarget,

    pub camera: Camera,
    resolution: UVec2,

    grid_renderer: grid_renderer::GridRenderer,
    debug_renderer: debug_renderer::DebugRenderer,
    skybox_renderer: skybox::SkyboxRenderer,

    light_buffer: glow::Buffer,
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

        let forward_program = PrimaryProgram::new(
            &gl,
            include_str!("shaders/tri.vert"),
            include_str!("shaders/tri.frag"),
        );

        let shadow_program = PrimaryProgram::new(
            &gl,
            include_str!("shaders/shadow_tri.vert"),
            include_str!("shaders/shadow_tri.frag"),
        );

        let skybox_renderer = skybox::SkyboxRenderer::new(&gl);

        let shadow_target = ShadowTarget::new(&gl, SHADOW_SIZE);

        let camera = Camera::default();

        let grid_renderer = grid_renderer::GridRenderer::new(&gl);
        let debug_renderer = debug_renderer::DebugRenderer::new(&gl);

        let resolution = UVec2::new(canvas.width(), canvas.height());

        let light_buffer = unsafe { gl.create_buffer().expect("Failed to create buffer") };

        Ok(Self {
            gl,
            canvas,
            forward_program,
            shadow_program,
            shadow_target,
            camera,
            resolution,
            grid_renderer,
            debug_renderer,
            skybox_renderer,
            light_buffer,
        })
    }

    pub fn resize(&mut self, dimension: UVec2) {
        self.resolution = dimension;
    }

    pub fn render(
        &self,
        draw_calls: &[DrawCall],
        debug_lines: &[DebugLine],
        lights: &[Light],
        grid_size: UVec3,
    ) {
        let aspect_ratio = self.canvas.client_width() as f32 / self.canvas.client_height() as f32;
        let light_direction = LIGHT_DIRECTION.normalize();

        unsafe {
            // Upload to light buffer
            let mut light_buffer_data = LightBuffer::default();
            for (idx, light) in lights.iter().enumerate().take(MAX_LIGHTS) {
                light_buffer_data.lights[idx] = *light;
            }
            light_buffer_data.light_count = lights.len() as u32;

            self.gl
                .bind_buffer(glow::UNIFORM_BUFFER, Some(self.light_buffer));
            self.gl.buffer_data_u8_slice(
                glow::UNIFORM_BUFFER,
                bytemuck::bytes_of(&light_buffer_data),
                glow::STREAM_DRAW,
            );

            let mut blend_state = EnableState::new(&self.gl, glow::BLEND, false);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ZERO,
            );

            self.gl
                .viewport(0, 0, self.resolution.x as i32, self.resolution.y as i32);

            self.gl.enable(glow::CULL_FACE);
            self.gl.cull_face(glow::BACK);

            self.gl.depth_func(glow::LEQUAL);

            // -------------------
            // --- Shadow Pass ---
            // -------------------

            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.shadow_target.framebuffer));
            self.gl
                .viewport(0, 0, SHADOW_SIZE.x as i32, SHADOW_SIZE.y as i32);
            self.gl.cull_face(glow::FRONT);

            self.gl.clear_depth_f32(1.0);
            self.gl.clear(glow::DEPTH_BUFFER_BIT);

            self.gl.enable(glow::POLYGON_OFFSET_FILL);
            self.gl.polygon_offset(0.0, 0.0);

            let shadow_from_world = compute_shadow_bounding_box(light_direction, grid_size);

            blend_state.set(&self.gl, false);

            self.render_pass(
                &self.shadow_program,
                draw_calls,
                None,
                shadow_from_world,
                None,
                light_direction,
            );

            // --------------------
            // --- Forward Pass ---
            // --------------------

            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl
                .viewport(0, 0, self.resolution.x as i32, self.resolution.y as i32);
            self.gl.cull_face(glow::BACK);

            // Set the clear color
            self.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.gl.disable(glow::POLYGON_OFFSET_FILL);

            let projection_matrix =
                Mat4::perspective_rh_gl(60.0_f32.to_radians(), aspect_ratio, 0.1, 100.0);

            let view_matrix = self.camera.view_matrix();

            let origin_view_matrix = self.camera.origin_view_matrix();

            let clip_from_world = projection_matrix * view_matrix;
            let origin_world_from_clip = (projection_matrix * origin_view_matrix).inverse();

            self.render_pass(
                &self.forward_program,
                draw_calls,
                Some(&mut blend_state),
                clip_from_world,
                Some(shadow_from_world),
                light_direction,
            );

            self.skybox_renderer
                .render(&self.gl, origin_world_from_clip);

            let clip_from_world = projection_matrix * view_matrix;
            self.grid_renderer
                .render(&self.gl, clip_from_world, UVec2::new(64, 64));

            self.debug_renderer
                .render(&self.gl, clip_from_world, debug_lines);

            self.gl.flush();
        }
    }

    fn render_pass(
        &self,
        program: &PrimaryProgram,
        draw_calls: &[DrawCall],
        blend_state: Option<&mut EnableState>,
        clip_from_world: Mat4,
        shadow_from_world: Option<Mat4>,
        light_direction: Vec3,
    ) {
        unsafe {
            self.gl.use_program(Some(program.program));

            self.gl
                .bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(self.light_buffer));

            for draw_call in draw_calls {
                let blending = draw_call.primitive.transparency_type.requires_blending();

                if let Some(&mut ref mut blend_state) = blend_state {
                    blend_state.set(&self.gl, blending);
                } else if blending {
                    // Skip this draw call if blending is required but we don't have a blend state
                    // as this means we are rendering to a target that doesn't support blending.
                    continue;
                }

                self.gl.depth_mask(!blending);

                // Set light direction
                self.gl.uniform_3_f32_slice(
                    program.light_dir_location.as_ref(),
                    (-light_direction).as_ref(),
                );

                // Set worldFromLocal matrix
                self.gl.uniform_matrix_4_f32_slice(
                    program.world_from_local_location.as_ref(),
                    false,
                    bytemuck::cast_slice(draw_call.transform.as_ref()),
                );

                let mvp_matrix = clip_from_world * draw_call.transform;

                // Set matrix
                self.gl.uniform_matrix_4_f32_slice(
                    program.matrix_location.as_ref(),
                    false,
                    bytemuck::cast_slice(mvp_matrix.as_ref()),
                );
                if let Some(shadow_from_world) = shadow_from_world {
                    let shadow_matrix = shadow_from_world * draw_call.transform;
                    self.gl.uniform_matrix_4_f32_slice(
                        program.shadow_matrix_location.as_ref(),
                        false,
                        bytemuck::cast_slice(slice::from_ref(&shadow_matrix)),
                    );
                }

                // Set tex
                self.gl.active_texture(glow::TEXTURE0);
                self.gl.bind_texture(
                    glow::TEXTURE_2D,
                    Some(draw_call.primitive.diffuse_texture.id),
                );

                // Set shadow map
                if let Some(_) = program.shadow_map_location.as_ref() {
                    self.gl.active_texture(glow::TEXTURE1);
                    self.gl
                        .bind_texture(glow::TEXTURE_2D, Some(self.shadow_target.depth_texture));
                }

                // Set tint
                let tint = draw_call.tint.unwrap_or(glam::Vec4::ONE);
                self.gl
                    .uniform_4_f32_slice(program.tint_location.as_ref(), tint.as_ref());

                // Set depth cutoff
                let depth_cutoff = draw_call.primitive.transparency_type.cutoff_value();
                self.gl
                    .uniform_1_f32(program.depth_cutoff_location.as_ref(), depth_cutoff);

                self.gl.bind_vertex_array(Some(draw_call.primitive.vao));

                self.gl.draw_elements(
                    glow::TRIANGLES,
                    draw_call.primitive.index_count as i32,
                    glow::UNSIGNED_INT,
                    draw_call.primitive.index_start as i32 * mem::size_of::<u32>() as i32,
                );
            }
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

struct PrimaryProgram {
    program: glow::Program,
    matrix_location: Option<glow::UniformLocation>,
    shadow_matrix_location: Option<glow::UniformLocation>,
    texture_location: Option<glow::UniformLocation>,
    tint_location: Option<glow::UniformLocation>,
    depth_cutoff_location: Option<glow::UniformLocation>,
    shadow_map_location: Option<glow::UniformLocation>,

    light_dir_location: Option<glow::UniformLocation>,
    world_from_local_location: Option<glow::UniformLocation>,
}

impl PrimaryProgram {
    fn new(gl: &glow::Context, vert_shader: &str, frag_shader: &str) -> Self {
        unsafe {
            let program = compile_shaders(gl, vert_shader, frag_shader);

            let matrix_location = gl.get_uniform_location(program, "matrix");
            let shadow_matrix_location = gl.get_uniform_location(program, "shadowMatrix");
            let texture_location = gl.get_uniform_location(program, "tex");
            let tint_location = gl.get_uniform_location(program, "tint");
            let depth_cutoff_location = gl.get_uniform_location(program, "depthCutoff");
            let shadow_map_location = gl.get_uniform_location(program, "shadowMap");

            let light_dir_location = gl.get_uniform_location(program, "lightDir");
            let world_from_local_location = gl.get_uniform_location(program, "worldFromLocal");

            let uniform_block_index = gl.get_uniform_block_index(program, "light_buffer");
            if let Some(uniform_block_index) = uniform_block_index {
                gl.uniform_block_binding(program, uniform_block_index, 0);
            }

            gl.use_program(Some(program));

            gl.uniform_1_i32(texture_location.as_ref(), 0);
            gl.uniform_1_i32(shadow_map_location.as_ref(), 1);

            Self {
                program,
                matrix_location,
                shadow_matrix_location,
                texture_location,
                tint_location,
                depth_cutoff_location,
                shadow_map_location,

                light_dir_location,
                world_from_local_location,
            }
        }
    }
}

struct ShadowTarget {
    framebuffer: glow::Framebuffer,
    depth_texture: glow::Texture,
}

impl ShadowTarget {
    fn new(gl: &glow::Context, size: UVec2) -> Self {
        let depth_texture = unsafe {
            let depth_texture = gl.create_texture().expect("Failed to create texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(depth_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::DEPTH_COMPONENT32F as i32,
                size.x as i32,
                size.y as i32,
                0,
                glow::DEPTH_COMPONENT,
                glow::FLOAT,
                None,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
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
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_COMPARE_MODE,
                glow::COMPARE_REF_TO_TEXTURE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_COMPARE_FUNC,
                glow::LEQUAL as i32,
            );
            depth_texture
        };

        let framebuffer = unsafe {
            let framebuffer = gl
                .create_framebuffer()
                .expect("Failed to create framebuffer");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::TEXTURE_2D,
                Some(depth_texture),
                0,
            );
            gl.draw_buffers(&[glow::NONE]);
            gl.read_buffer(glow::NONE);
            tracing::error!("{:X?}", gl.check_framebuffer_status(glow::FRAMEBUFFER));

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            framebuffer
        };

        Self {
            framebuffer,
            depth_texture,
        }
    }

    fn dispose(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_framebuffer(self.framebuffer);
            gl.delete_texture(self.depth_texture);
        }
    }
}
// CRIME(cw): This isn't really a crime but minecraft has this exact class and I feel nasty.
struct EnableState {
    kind: u32,
    enabled: bool,
}

impl EnableState {
    fn new(gl: &glow::Context, kind: u32, enabled: bool) -> Self {
        unsafe {
            if enabled {
                gl.enable(kind);
            } else {
                gl.disable(kind);
            }
        }
        Self { kind, enabled }
    }

    fn set(&mut self, gl: &glow::Context, enabled: bool) {
        if self.enabled != enabled {
            unsafe {
                if enabled {
                    gl.enable(self.kind);
                } else {
                    gl.disable(self.kind);
                }
            }
            self.enabled = enabled;
        }
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
    transparency_type: TransparencyType,
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
                transparency_type: primitive.material.transparency_type,
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
    tint: Option<glam::Vec4>,
) -> Vec<DrawCall> {
    let mut render_objects = Vec::new();

    for (idx, model) in models.iter().enumerate() {
        build_render_plan_recursive(
            &mut render_objects,
            model,
            &render_model[idx],
            model.root_node_idx,
            transform,
            tint,
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
    tint: Option<glam::Vec4>,
) {
    let node = &gltf.nodes[current_node];

    let transform = parent_transform * node.current_transform;

    if let Some(mesh) = node.mesh {
        let render_mesh = &render_model.meshes[mesh];
        for primitive in &render_mesh.primitives {
            draw_calls.push(DrawCall {
                primitive: primitive.clone(),
                transform: transform.as_affine().into(),
                tint,
            });
        }
    }

    for &child in &node.children {
        build_render_plan_recursive(draw_calls, gltf, render_model, child, transform, tint);
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

    /// View matrix without any translation. Used For skybox stuff
    pub fn origin_view_matrix(&self) -> Mat4 {
        Mat4::from_quat(self.rotation).inverse()
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
    blocks: impl IntoIterator<Item = (BlockPos, &'a [Texture; 6])>,
    transparency_type: TransparencyType,
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
                    transparency_type,
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

pub fn compute_shadow_bounding_box(direction: Vec3, grid_size: UVec3) -> Mat4 {
    // TODO(cw): Too Fancy
    // // All 8 corners of the grid
    // let corners = [
    //     Vec3::new(0.0, 0.0, 0.0),
    //     Vec3::new(grid_size.x as f32, 0.0, 0.0),
    //     Vec3::new(0.0, grid_size.y as f32, 0.0),
    //     Vec3::new(grid_size.x as f32, grid_size.y as f32, 0.0),
    //     Vec3::new(0.0, 0.0, grid_size.z as f32),
    //     Vec3::new(grid_size.x as f32, 0.0, grid_size.z as f32),
    //     Vec3::new(0.0, grid_size.y as f32, grid_size.z as f32),
    //     Vec3::new(grid_size.x as f32, grid_size.y as f32, grid_size.z as f32),
    // ];

    // // Project those corners in the direction of the light
    // let zero_view_matrix = Mat4::look_at_rh(-direction, Vec3::ZERO, Vec3::Y);

    // let mut min = Vec3::INFINITY;
    // let mut max = Vec3::NEG_INFINITY;

    // for corner in corners {
    //     let projected = zero_view_matrix.transform_point3(corner);

    //     min = min.min(projected);
    //     max = max.max(projected);
    // }

    // let center = (min + max) / 2.0;
    // let size = max - min;

    let center = grid_size.as_vec3() * 0.5;
    let size = Vec3::splat(grid_size.as_vec3().max_element() * 1.5);

    tracing::info!("Center: {:?}, Size: {:?}", center, size);

    let projection_matrix = Mat4::orthographic_rh_gl(
        -size.x / 2.0,
        size.x / 2.0,
        -size.y / 2.0,
        size.y / 2.0,
        -size.z / 2.0,
        size.z / 2.0,
    );

    let view_matrix = Mat4::look_at_rh(center - direction, center, Vec3::Y);

    projection_matrix * view_matrix
}
