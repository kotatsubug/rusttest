extern crate gl;
extern crate sdl2;
extern crate thiserror;
extern crate winapi;
extern crate glam;

pub mod gfx;
pub mod math;
pub mod system;
pub mod resource;
pub mod log;

use log::LOGGER;
use math::AffineTransform;

extern "system" fn gl_debug_message_callback(
    source: u32, ty: u32, id: u32, severity: u32, length: i32,
    message: *const std::os::raw::c_char, user_param: *mut std::os::raw::c_void)
{
    let _ = (source, ty, id, severity, user_param);

    match severity {
        gl::DEBUG_SEVERITY_HIGH | gl::DEBUG_SEVERITY_MEDIUM | gl::DEBUG_SEVERITY_LOW => {
            unsafe {
                let message = std::slice::from_raw_parts(message as *const u8, length as usize);
                let message = std::str::from_utf8(message);
                match message {
                    Ok(m) => {
                        LOGGER().a.warn(
                            format!("OpenGL callback: {}", m).as_str()
                        );
                    }
                    Err(e) => {
                        LOGGER().a.error(
                            format!("received invalid OpenGL callback message: {}", e.to_string()).as_str()
                        );
                    }
                }
                
            }
        }
        gl::DEBUG_SEVERITY_NOTIFICATION | _ => {}
    }
}

fn run() {
    match LOGGER().a.set_log_path("debug.log") {
        Err(e) => LOGGER().a.error(&e),
        _ => {}
    }

    let res = resource::Resource::from_relative_exe_path(std::path::Path::new("assets")).unwrap();

    let sdl = sdl2::init().expect("could not initialize SDL");
    let video_subsys = sdl.video().expect("could not initialize SDL video subsystem");
    
    let mut input = system::InputDevice::new(&sdl);
    
    let gl_attr = video_subsys.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 3);
    gl_attr.set_accelerated_visual(true);
    gl_attr.set_double_buffer(true);

    let window = video_subsys
        .window("WINDOW_TITLE", 640, 480)
        .opengl()
        .resizable()
        .allow_highdpi()
        .build()
        .expect("could not build SDL window");
    
    let _gl_context = window.gl_create_context().expect("could not create OpenGL context for SDL window");
    let _gl = gl::load_with(|s| video_subsys.gl_get_proc_address(s) as *const _);

    let vsync = false;
    match video_subsys.gl_set_swap_interval(if vsync { 1 } else { 0 }) {
        Err(e) => {
            LOGGER().a.error(format!("failed to set swap interval: {}", e).as_str());
        },
        _ => {}
    };
    
    let mut vendor_info: String = "".to_owned();
    vendor_info.push_str(
        unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::VENDOR) as *const i8).to_str().unwrap() }
    );
    vendor_info.push_str(" ");
    vendor_info.push_str(
        unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::RENDERER) as *const i8).to_str().unwrap() }
    );
    LOGGER().a.info(&vendor_info);
    let gl_version_info: String = 
        unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as *const i8).to_str().unwrap().to_string() };
    LOGGER().a.info(format!("using OpenGL version {}", &gl_version_info).as_str());
    LOGGER().a.info(format!("using SDL2 version {}", sdl2::version::version().to_string()).as_str());

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
    let mut transforms: Vec<glam::Mat4> = vec![
        glam::Mat4::IDENTITY,
    ];

    let mut batch = gfx::Batch::new(program.id(), mesh, &transforms).unwrap();
    
    let mut view: glam::Mat4 = glam::Mat4::IDENTITY;
    let mut projection: glam::Mat4 = glam::Mat4::perspective_lh(
        90.0,
        viewport.width as f32 / viewport.height as f32,
        0.01,
        100.0
    );
    let mut camera_transform = AffineTransform::new(
        glam::vec3(0.0, 0.0, -1.0),
        glam::Quat::from_axis_angle(glam::vec3(0.0, 1.0, 0.0), 0.0),
        glam::vec3(0.0, 0.0, 0.0),
    );
    let mut camera = gfx::Camera::new(view, projection, camera_transform, glam::vec3(0.0, 1.0, 0.0));
    
    let mut event_pump = sdl.event_pump()
        .expect("attempted to obtain SDL event pump when an EventPump instance already exists");
    'main_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit {..} => {
                    break 'main_loop;
                },
                sdl2::event::Event::Window { win_event: sdl2::event::WindowEvent::Resized(w, h), .. } => {
                    viewport.update_size(w, h);
                    viewport.use_viewport();
                    
                    camera.projection = glam::Mat4::perspective_lh(
                        90.0,
                        viewport.width as f32 / viewport.height as f32,
                        0.01,
                        100.0
                    );
                }
                _ => {},
            }
        }

        input.process_keymap(&event_pump);
        input.process_mousemap(&event_pump);

        if input.is_key_down(&sdl2::keyboard::Keycode::Escape) {
            break 'main_loop;
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        program.use_program();
        
        program.set_mat4fv("View", camera.view, 0);
        program.set_mat4fv("Projection", camera.projection, 0);

        batch.draw();

        if input.is_key_down(&sdl2::keyboard::Keycode::W) {
            camera.transform.position.z += 0.0004;
        }
        if input.is_key_down(&sdl2::keyboard::Keycode::S) {
            camera.transform.position.z -= 0.0004;
        }
        if input.is_key_down(&sdl2::keyboard::Keycode::A) {
            camera.transform.position.x -= 0.0004;
        }
        if input.is_key_down(&sdl2::keyboard::Keycode::D) {
            camera.transform.position.x += 0.0004;
        }

        camera.update_view();

        window.gl_swap_window();
    }
}

fn main() -> Result<(), String> {
    let _args: Vec<_> = std::env::args().collect();

    let r = std::panic::catch_unwind(|| {
        run();
    });

    let r_str: Option<String> = match r {
        Ok(_) => None,
        Err(e) => {
            let panic_info = match e.downcast::<String>() {
                Ok(v) => *v,
                Err(e) => match e.downcast::<&str>() {
                    Ok(v) => v.to_string(),
                    _ => "Unknown source of error".to_owned(),
                }
            };

            Some(format!("{}\n", panic_info).to_string())
        },
    };

    if r_str.is_some() {
        LOGGER().a.fatal(r_str.as_ref().unwrap());
        match system::windows::create_message_box("Engine Panic", &r_str.unwrap(), system::windows::IconType::None) {
            Err(e) => { LOGGER().a.error(format!("{}", &e).as_str()); },
            _ => {},
        }
    }

    // make sure buffers don't do anything weird to the log file as it is saved
    // if this point isn't reached on thread panic, you probably have bigger problems to worry about
    LOGGER().a.flush().unwrap();

    Ok(())
}