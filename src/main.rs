extern crate gl;
extern crate sdl2;
extern crate thiserror;
extern crate winapi;

pub mod gfx;

pub mod resource;
pub mod log;

use log::LOGGER;

use crate::resource::Resource;

fn main() {
    match LOGGER().a.set_log_path("debug.log") {
        Err(e) => LOGGER().a.error(&e),
        _ => {}
    }
    LOGGER().a.debug("Hello!");

    let sdl = sdl2::init().unwrap();
    let video_subsys = sdl.video().unwrap();

    let gl_attr = video_subsys.gl_attr();

    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 3);

    let window = video_subsys
        .window("WINDOW_TITLE", 640, 480)
        .opengl()
        .resizable()
        .build()
        .unwrap();
    
    let _gl_context = window.gl_create_context().unwrap();
    let _gl = gl::load_with(|s| video_subsys.gl_get_proc_address(s) as *const std::os::raw::c_void);

    let mut vendor_info: String = ("").to_owned();
    vendor_info.push_str(
        unsafe {
            std::ffi::CStr::from_ptr(gl::GetString(gl::VENDOR) as *const i8).to_str().unwrap()
        }
    );
    vendor_info.push_str(" ");
    vendor_info.push_str(
        unsafe {
            std::ffi::CStr::from_ptr(gl::GetString(gl::RENDERER) as *const i8).to_str().unwrap()
        }
    );
    LOGGER().a.info(&vendor_info);
    
    let mut viewport = gfx::Viewport::make_viewport(640, 480);
    
    unsafe {
        gl::ClearColor(0.3, 0.3, 0.5, 1.0);
    }

    let res = Resource::from_relative_exe_path(std::path::Path::new("assets")).unwrap();
    let program = gfx::Program::from_res(&res, "shaders/test").unwrap();
    
    let vertices: Vec<f32> = vec![
        -0.5, -0.5, 0.0,
        0.5, -0.5, 0.0,
        0.0, 0.5, 0.0
    ];

    let mut vbo: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(gl::ARRAY_BUFFER, (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr, vertices.as_ptr() as *const gl::types::GLvoid, gl::STATIC_DRAW);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }

    let mut vao: gl::types::GLuint = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
    }

    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::EnableVertexAttribArray(0); // this is "layout (location = 0)" in vertex shader
        gl::VertexAttribPointer(
            0, // index of the generic vertex attribute ("layout (location = 0)")
            3, // the number of components per generic vertex attribute
            gl::FLOAT, // data type
            gl::FALSE, // normalized (int-to-float conversion)
            (3 * std::mem::size_of::<f32>()) as gl::types::GLint, // stride (byte offset between consecutive attributes)
            std::ptr::null() // offset of the first component
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
    }

    let mut event_pump = sdl.event_pump().unwrap();
    'main_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit {..} => {
                    break 'main_loop;
                }
                sdl2::event::Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h), ..
                } => {
                    viewport.update_size(w, h);
                    viewport.use_viewport();
                }
                _ => {},
            }
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        program.use_program();
        unsafe {
            gl::BindVertexArray(vao);
            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                3
            );
        }

        window.gl_swap_window();
    }

    LOGGER().a.flush().unwrap();
}