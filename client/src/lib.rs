use glow::HasContext;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

// Enable console.log for debugging
macro_rules! console_log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

// Entry point when the module is loaded
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Access the canvas element
    let window = web_sys::window().ok_or("Could not access window")?;
    let document = window.document().ok_or("Could not access document")?;
    let canvas = document
        .get_element_by_id("canvas")
        .ok_or("Canvas element not found")?;
    let canvas: HtmlCanvasElement = canvas.dyn_into::<HtmlCanvasElement>()?;

    // Get the WebGL2 rendering context
    let webgl2_context = canvas
        .get_context("webgl2")?
        .ok_or("WebGL2 not supported")?
        .dyn_into::<WebGl2RenderingContext>()?;

    // Initialize glow with the WebGL2 context
    let gl = glow::Context::from_webgl2_context(webgl2_context);

    unsafe {
        // Set the clear color
        gl.clear_color(0.1, 0.1, 0.1, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        // Define the triangle vertices
        let vertices: [f32; 6] = [
            0.0, 0.5, // Vertex 1 (X, Y)
            0.5, -0.5, // Vertex 2 (X, Y)
            -0.5, -0.5, // Vertex 3 (X, Y)
        ];

        // Create and bind the vertex buffer
        let vertex_buffer = gl.create_buffer().expect("Failed to create buffer");
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STATIC_DRAW,
        );

        // Vertex shader source
        let vertex_shader_source = r#"
            attribute vec2 position;
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "#;

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
            return Err(JsValue::from_str(&log));
        }

        // Fragment shader source
        let fragment_shader_source = r#"
            void main() {
                gl_FragColor = vec4(1.0, 0.5, 0.2, 1.0);
            }
        "#;

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
            return Err(JsValue::from_str(&log));
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
            return Err(JsValue::from_str(&log));
        }

        // Use the program and set up the vertex attributes
        gl.use_program(Some(program));

        let position_attribute_location = gl
            .get_attrib_location(program, "position")
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

        // Clean up (optional, as the program will end here)
        gl.disable_vertex_attrib_array(position_attribute_location);
        gl.use_program(None);
        gl.delete_program(program);
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
        gl.delete_buffer(vertex_buffer);
    }

    Ok(())
}
