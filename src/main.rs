extern crate gl;
extern crate sdl2;
extern crate thiserror;
extern crate winapi;
extern crate glam;

pub mod gfx;
pub mod math;
pub mod resource;
pub mod log;

use log::LOGGER;

extern "system" fn gl_debug_message_callback(
    source: u32, ty: u32, id: u32, severity: u32, length: i32,
    message: *const std::os::raw::c_char, user_param: *mut std::os::raw::c_void)
{
    let _ = (source, ty, id, severity, user_param);

    if severity != gl::DEBUG_SEVERITY_NOTIFICATION {
        unsafe {
            let message = std::slice::from_raw_parts(message as *const u8, length as usize);
            let message = std::str::from_utf8(message).expect("bad opengl error message");
            LOGGER().a.debug(message);
        }
    }
}

fn main() {
    match LOGGER().a.set_log_path("debug.log") {
        Err(e) => LOGGER().a.error(&e),
        _ => {}
    }

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
    let _gl = gl::load_with(|s| video_subsys.gl_get_proc_address(s) as *const _);
    
    let mut vendor_info: String = "".to_owned();
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

    unsafe {
        gl::Enable(gl::DEBUG_OUTPUT);
        gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
        gl::DebugMessageCallback(Some(gl_debug_message_callback), std::ptr::null());
        gl::DebugMessageControl(gl::DONT_CARE, gl::DONT_CARE, gl::DONT_CARE, 0, std::ptr::null(), gl::TRUE);
    }
    
    let mut viewport = gfx::Viewport::make_viewport(640, 480);
    
    unsafe {
        gl::ClearColor(0.3, 0.3, 0.5, 1.0);
    }

    let res = resource::Resource::from_relative_exe_path(std::path::Path::new("assets")).unwrap();
    let program = gfx::Program::from_res(&res, "shaders/test").unwrap();

    let vertices: Vec<gfx::Vertex> = vec![
        gfx::Vertex {
            pos: (0.5, -0.5, 0.0).into(),
            color: (1.0, 0.0, 1.0).into()
        },
        gfx::Vertex {
            pos: (-0.5, -0.5, 0.0).into(),
            color: (0.0, 1.0, 1.0).into()
        },
        gfx::Vertex {
            pos: (0.0, 0.5, 0.0).into(),
            color: (1.0, 1.0, 0.0).into()
        },
    ];
    let indices: Vec<u32> = vec![
        0, 1, 2
    ];
    let mesh = gfx::Mesh::new(vertices, indices);
    let transforms: Vec<glam::Mat4> = vec![
        glam::Mat4::IDENTITY,
    ];

    let batch = gfx::Batch::new(program.id(), mesh, transforms).unwrap();

    let view: glam::Mat4 = glam::Mat4::IDENTITY;
    let projection: glam::Mat4 = glam::Mat4::IDENTITY;

    let mut event_pump = sdl.event_pump().unwrap();
    'main_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit {..} => {
                    break 'main_loop;
                }
                sdl2::event::Event::Window { win_event: sdl2::event::WindowEvent::Resized(w, h), .. } => {
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
        
        program.set_mat4fv("View", view, 0);
        program.set_mat4fv("Projection", projection, 0);

        batch.draw();

        window.gl_swap_window();
    }
    
    LOGGER().a.flush().unwrap();
}