// Copyright 2015 Brendan Zabarauskas and the gl-rs developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate gl;
extern crate glutin;

use gl::types::*;
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::str;
use std::time::Instant;

// Vertex data
static VERTEX_DATA: [GLfloat; 8] = [
    -1.0, 1.0,
    -1.0, -1.0,
    1.0, 1.0,
    1.0, -1.0
];

// Shader sources
static VS_SRC: &'static str = "
#version 150
in vec2 in_position;

void main() {
    gl_Position = vec4(in_position, 0.0, 1.0);
}";

static FS_SRC: &'static str = "
#version 150

uniform vec2 u_resolution;
uniform float u_time;
out vec4 out_color;

void mainImage(out vec4 e)
{
    vec3 d=.5-vec3(gl_FragCoord.xy,1)/u_resolution.y,p,o;
    for(int i=0;i<32;i++)
    {
        o=p;
        o.z-=u_time*9.;
        float a=o.z*.1;
        o.xy*=mat2(cos(a),sin(a),-sin(a),cos(a));
        p+=(.1-length(cos(o.xy)+sin(o.yz)))*d;
    }
    e=vec4((sin(p)+vec3(2,5,9))/length(p)*vec3(1), 1);
}

void main() {
    mainImage(out_color);
}
";

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = gl::CreateShader(ty);
        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf)
                    .ok()
                    .expect("ShaderInfoLog not valid utf8")
            );
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf)
                    .ok()
                    .expect("ProgramInfoLog not valid utf8")
            );
        }
        program
    }
}

fn main() {
    let now = Instant::now();
    let time_uniform;
    let resolution_uniform;
    let event_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new();
    let gl_window = glutin::ContextBuilder::new()
        .build_windowed(window, &event_loop)
        .unwrap();

    // It is essential to make the context current before calling `gl::load_with`.
    let gl_window = unsafe { gl_window.make_current() }.unwrap();

    // Load the OpenGL function pointers
    gl::load_with(|symbol| gl_window.get_proc_address(symbol));

    // Create GLSL shaders
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    let program = link_program(vs, fs);

    let mut vao = 0;
    let mut vbo = 0;

    unsafe {
        // Create Vertex Array Object
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Create a Vertex Buffer Object and copy the vertex data to it
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            mem::transmute(&VERTEX_DATA[0]),
            gl::STATIC_DRAW,
        );

        // Use shader program
        gl::UseProgram(program);
        gl::BindFragDataLocation(program, 0, CString::new("out_color").unwrap().as_ptr());

        time_uniform = gl::GetUniformLocation(program, CString::new("u_time").unwrap().as_ptr());
        resolution_uniform = gl::GetUniformLocation(program, CString::new("u_resolution").unwrap().as_ptr());

        // Specify the layout of the vertex data
        let pos_attr = gl::GetAttribLocation(program, CString::new("in_position").unwrap().as_ptr());
        gl::EnableVertexAttribArray(pos_attr as GLuint);
        gl::VertexAttribPointer(
            pos_attr as GLuint,
            2,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            0,
            ptr::null(),
        );
    }

    event_loop.run(move |event, _, control_flow| {
        use glutin::event::{Event, WindowEvent};
        use glutin::event_loop::ControlFlow;
        *control_flow = ControlFlow::Poll;
        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    // Cleanup
                    unsafe {
                        gl::DeleteProgram(program);
                        gl::DeleteShader(fs);
                        gl::DeleteShader(vs);
                        gl::DeleteBuffers(1, &vbo);
                        gl::DeleteVertexArrays(1, &vao);
                    }
                    *control_flow = ControlFlow::Exit
                },
                _ => (),
            },
            // Event::RedrawRequested(_) => {
            // },
            _ => (),
        };
        unsafe {
            let window_size = gl_window.window().inner_size();
            // Clear the screen to black
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            // Update uniforms
            gl::Uniform1f(time_uniform, now.elapsed().as_secs_f32());
            gl::Uniform2f(resolution_uniform, window_size.width as f32, window_size.height as f32);
            // Draw a triangle from the 3 vertices
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        }
        gl_window.swap_buffers().unwrap();
    });
}
