use std::{mem, slice, time::Duration};

use bytemuck::offset_of;
use glam::{Mat4, Vec3};
use glow::HasContext;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

use crate::{
    gltf::{GLTFPrimitive, GLTFVertex},
    transform::Transform,
};

const POSITION_ATTRIBUTE: u32 = 0;
const NORMAL_ATTRIBUTE: u32 = 1;
const UV_ATTRIBUTE: u32 = 2;

pub struct Renderer {
    gl: glow::Context,

    program: glow::Program,
    matrix_location: Option<glow::UniformLocation>,

    pub camera: Camera,
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

        let camera = Camera::default();

        Ok(Self {
            gl,
            program,
            matrix_location,
            camera,
        })
    }

    pub fn render(&self, elapsed_time: Duration, draw_calls: &[DrawCall]) {
        let gl = &self.gl;

        unsafe {
            // Set the clear color
            gl.clear_color(0.1, 0.1, 0.1, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            let projection_matrix =
                Mat4::perspective_rh_gl(45.0_f32.to_radians(), 800.0 / 600.0, 0.1, 100.0);

            let view_matrix = self.camera.view_matrix();

            gl.use_program(Some(self.program));

            for draw_call in draw_calls {
                let mvp_matrix = projection_matrix * view_matrix * draw_call.transform;

                gl.uniform_matrix_4_f32_slice(
                    self.matrix_location.as_ref(),
                    false,
                    bytemuck::cast_slice(slice::from_ref(&mvp_matrix)),
                );

                gl.bind_vertex_array(Some(draw_call.primitive.vao));
                gl.bind_texture(glow::TEXTURE_2D, Some(draw_call.primitive.diffuse_texture));
                gl.draw_elements(
                    glow::TRIANGLES,
                    draw_call.primitive.index_count as i32,
                    glow::UNSIGNED_INT,
                    0,
                );
            }
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
    diffuse_texture: glow::Texture,
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

            let stride = mem::size_of::<crate::gltf::GLTFVertex>() as i32;

            gl.enable_vertex_attrib_array(POSITION_ATTRIBUTE);
            gl.vertex_attrib_pointer_f32(
                POSITION_ATTRIBUTE,
                3,
                glow::FLOAT,
                false,
                stride,
                offset_of!(GLTFVertex, position) as i32,
            );

            gl.enable_vertex_attrib_array(NORMAL_ATTRIBUTE);
            gl.vertex_attrib_pointer_f32(
                NORMAL_ATTRIBUTE,
                3,
                glow::FLOAT,
                false,
                stride,
                offset_of!(GLTFVertex, normal) as i32,
            );

            gl.enable_vertex_attrib_array(UV_ATTRIBUTE);
            gl.vertex_attrib_pointer_f32(
                UV_ATTRIBUTE,
                2,
                glow::FLOAT,
                false,
                stride,
                offset_of!(GLTFVertex, uv) as i32,
            );

            gl.bind_vertex_array(None);

            let diffuse_texture = gl.create_texture().expect("Failed to create texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(diffuse_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                primitive.material.base_colour_texture.dimensions.x as i32,
                primitive.material.base_colour_texture.dimensions.y as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(&primitive.material.base_colour_texture.data),
            );

            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR_MIPMAP_LINEAR as i32,
            );

            Self {
                vao,
                diffuse_texture,
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

    let transform = parent_transform * node.transform;

    if let Some(mesh) = node.mesh {
        let render_mesh = &render_model.meshes[mesh];
        for primitive in &render_mesh.primitives {
            draw_calls.push(DrawCall {
                primitive: primitive.clone(),
                transform: transform.as_affine().into(),
            });
        }
    }

    for &child in &node.children {
        build_render_plan_recursive(draw_calls, gltf, render_model, child, transform);
    }
}

#[derive(Debug)]
pub struct DrawCall {
    primitive: RenderPrimitive,
    transform: glam::Mat4,
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
    pub translation: Vec3,
    pub rotation: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            translation: Vec3::new(0.0, 0.0, -5.0),
            rotation: Default::default(),
        }
    }
}

impl Camera {
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
        ) * Mat4::from_translation(self.translation)
    }
}
