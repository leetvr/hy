use std::time::Duration;

use glow::HasContext;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

use crate::console_log;

pub struct Renderer {
    gl: glow::Context,

    program: glow::Program,
}

impl Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        // Get the WebGL2 rendering context
        let webgl2_context = canvas
            .get_context("webgl2")?
            .ok_or("WebGL2 not supported")?
            .dyn_into::<WebGl2RenderingContext>()?;

        // Initialize glow with the WebGL2 context
        let gl = glow::Context::from_webgl2_context(webgl2_context);

        let vertex_shader_source = include_str!("shaders/tri.vert");
        let fragment_shader_source = include_str!("shaders/tri.frag");

        let program = compile_shaders(&gl, vertex_shader_source, fragment_shader_source);

        Ok(Self { gl, program })
    }

    pub fn render(&self, elapsed_time: Duration) {
        let gl = &self.gl;

        unsafe {
            // Set the clear color
            gl.clear_color(0.1, 0.1, 0.1, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            let effect = (elapsed_time.as_secs_f32().sin() * 0.5 + 0.5) * 0.25;

            // Define the triangle vertices
            let vertices: [f32; 6] = [
                0.0 + effect,
                0.5, // Vertex 1 (X, Y)
                0.5 + effect,
                -0.5, // Vertex 2 (X, Y)
                -0.5 + effect,
                -0.5, // Vertex 3 (X, Y)
            ];

            // Create and bind the vertex buffer
            let vertex_buffer = gl.create_buffer().expect("Failed to create buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            // Use the program and set up the vertex attributes
            gl.use_program(Some(self.program));

            let position_attribute_location = gl
                .get_attrib_location(self.program, "position")
                .expect("gl: Unable to get position");

            gl.enable_vertex_attrib_array(position_attribute_location);
            gl.vertex_attrib_pointer_f32(
                position_attribute_location,
                2,           // size
                glow::FLOAT, // type
                false,       // normalized
                0,           // stride
                0,           // offset
            );

            // Draw the triangle
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }
}

fn compile_shaders(
    gl: &glow::Context,
    vertex_shader_source: &str,
    fragment_shader_source: &str,
) -> glow::WebProgramKey {
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
            console_log!("Vertex shader compilation failed: {}", log);
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
            console_log!("Fragment shader compilation failed: {}", log);
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
            console_log!("Program linking failed: {}", log);
            // return Err(JsValue::from_str(&log));
            panic!("Program linking failed: {}", log);
        }

        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
        program
    }
}
