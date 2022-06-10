use crate::resource::Resource;
use crate::log::LOGGER;

use std::collections::HashMap;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to load resource {}", name)]
    ResourceLoadError {
        name: String,
        inner: crate::resource::Error
    },
    #[error("Can not determine shader type for resource {}", name)]
    CanNotDetermineShaderTypeForResource {
        name: String
    },
    #[error("Failed to compile shader {}: {}", name, message)]
    CompileError {
        name: String,
        message: String
    },
    #[error("Failed to link program {}: {}", name, message)]
    LinkError {
        name: String,
        message: String
    },
}

pub struct Program {
    id: gl::types::GLuint,
    uniforms: HashMap<String, UniformInfo>,
}

pub struct Shader {
    id: gl::types::GLuint,
}

struct UniformInfo {
    location: gl::types::GLint,
    count: gl::types::GLsizei,
}

impl Program {
    pub fn from_res(res: &Resource, name: &str) -> Result<Program, Error> {
        const POSSIBLE_EXTENSIONS: [&str; 2] = [".vert", ".frag"];

        let resource_names = POSSIBLE_EXTENSIONS
            .iter()
            .map(|file_extension| format!("{}{}", name, file_extension))
            .collect::<Vec<String>>();
        
        let shaders = resource_names
            .iter()
            .map(|resource_name| Shader::from_res(res, resource_name))
            .collect::<Result<Vec<Shader>, Error>>()?;
        
        Program::from_shaders(&shaders[..]).map_err(|message| Error::LinkError {
            name: name.into(),
            message,
        })
    }

    pub fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl::CreateProgram() };
        
        for shader in shaders {
            unsafe { gl::AttachShader(program_id, shader.id()); }
        }

        unsafe { gl::LinkProgram(program_id); }

        let mut success: gl::types::GLint = 1;
        unsafe { gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success); }

        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe { gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len); }
            
            let error = create_whitespace_cstring_with_len(len as usize);
            unsafe {
                gl::GetProgramInfoLog(program_id, len, std::ptr::null_mut(), error.as_ptr() as *mut gl::types::GLchar);
            }

            return Err(error.to_string_lossy().into_owned());
        }

        for shader in shaders {
            unsafe { gl::DetachShader(program_id, shader.id()); }
        }

        Ok(Program {
            id: program_id,
            uniforms: Program::build_uniform_map(program_id)
        })
    }
    
    /// Returns a `HashMap` of uniform names to their respective `location` and `count` in the program, 
    /// since manually parsing the shader source strings to retrieve uniform information is slow and horrible and 
    /// people need to stop doing it
    /// 
    /// Returns an empty HashMap if there are no active uniforms for this program.
    fn build_uniform_map(program_id: gl::types::GLuint) -> HashMap<String, UniformInfo> {
        let mut uniform_count: i32 = 0;
        unsafe { gl::GetProgramiv(program_id, gl::ACTIVE_UNIFORMS, &mut uniform_count); }

        let mut uniforms: HashMap<String, UniformInfo> = HashMap::new();
        if uniform_count != 0 {
            let mut max_name_len: gl::types::GLint = 0;
            let mut length: gl::types::GLsizei = 0;
            let mut count: gl::types::GLsizei = 0;
            let mut type_: gl::types::GLenum = gl::NONE;
            unsafe { gl::GetProgramiv(program_id, gl::ACTIVE_UNIFORM_MAX_LENGTH, &mut max_name_len); }
            
            for i in 0..uniform_count as u32 {
                unsafe {
                    let uniform_name_empty = create_whitespace_cstring_with_len(max_name_len as usize);
                    let uniform_name_ptr = uniform_name_empty.into_raw();
                    
                    gl::GetActiveUniform(
                        program_id,
                        i,
                        max_name_len,
                        &mut length,
                        &mut count,
                        &mut type_,
                        uniform_name_ptr
                    );

                    let uniform_info = UniformInfo{
                        location: gl::GetUniformLocation(program_id, uniform_name_ptr),
                        count: count,
                    };

                    let uniform_name_cstr = std::ffi::CString::from_raw(uniform_name_ptr);
                    let uniform_name = std::ffi::CString::into_string(uniform_name_cstr).unwrap();

                    LOGGER().a.debug(
                        format!(
                            "added uniform '{}' (location={}) (count={}) to program {} uniforms map",
                            uniform_name,
                            uniform_info.location,
                            uniform_info.count,
                            program_id
                        ).as_str()
                    );

                    uniforms.insert(
                        uniform_name,
                        uniform_info,
                    );
                }
            }
        } else {
            LOGGER().a.warn(
                format!(
                    "program {} reports no active uniforms when building uniform map for its shaders!",
                    program_id
                ).as_str()
            );
        }

        uniforms
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }

    pub fn use_program(&self) {
        unsafe { gl::UseProgram(self.id); }
    }

    #[inline(always)]
    pub fn set_i32(&self, uniform_name: &str, value: i32) {
        unsafe { gl::ProgramUniform1i(self.id, self.uniforms.get(uniform_name).unwrap().location, value); }
    }

    #[inline(always)]
    pub fn set_f32(&self, uniform_name: &str, value: f32) {
        unsafe { gl::ProgramUniform1f(self.id, self.uniforms.get(uniform_name).unwrap().location, value); }
    }

    #[inline(always)]
    pub fn set_vec2f(&self, uniform_name: &str, value: glam::Vec2) {
        unsafe { gl::ProgramUniform2f(self.id, self.uniforms.get(uniform_name).unwrap().location,
            value.x, value.y); }
    }

    #[inline(always)]
    pub fn set_vec3f(&self, uniform_name: &str, value: glam::Vec3) {
        unsafe { gl::ProgramUniform3f(self.id, self.uniforms.get(uniform_name).unwrap().location,
            value.x, value.y, value.z); }
    }

    #[inline(always)]
    pub fn set_vec4f(&self, uniform_name: &str, value: glam::Vec4) {
        unsafe { gl::ProgramUniform4f(self.id, self.uniforms.get(uniform_name).unwrap().location,
            value.x, value.y, value.z, value.w); }
    }

    #[inline(always)]
    pub fn set_mat4fv(&self, uniform_name: &str, value: glam::Mat4, transpose: gl::types::GLboolean) {
        unsafe {
            match self.uniforms.get(uniform_name) {
                Some(p) => {
                    gl::ProgramUniformMatrix4fv(self.id, p.location,
                        1, transpose, &value.to_cols_array()[0]);
                },
                _ => {
                    LOGGER().a.error(format!(
                        "attempted to set uniform '{}' but it doesn't exist in the uniform map!", uniform_name
                    ).as_str());
                }
            }
            
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id); }
    }
}

impl Shader {
    pub fn from_res(res: &Resource, name: &str) -> Result<Shader, Error> {
        const POSSIBLE_EXTENSIONS: [(&str, gl::types::GLenum); 2] = 
            [(".vert", gl::VERTEX_SHADER), (".frag", gl::FRAGMENT_SHADER)];

        let shader_kind = POSSIBLE_EXTENSIONS
            .iter()
            .find(|&&(file_extension, _)| name.ends_with(file_extension))
            .map(|&(_, kind)| kind)
            .ok_or_else(|| Error::CanNotDetermineShaderTypeForResource { name: name.into() })?;
        
        let source = res.load_cstring(name).map_err(|e| Error::ResourceLoadError {
            name: name.into(),
            inner: e,
        })?;

        Shader::from_source(&source, shader_kind).map_err(|message| Error::CompileError {
            name: name.into(),
            message,
        })
    }

    pub fn from_source(source: &std::ffi::CStr, kind: gl::types::GLenum) -> Result<Shader, String> {
        let id = shader_from_source(source, kind)?;

        Ok(Shader { id })
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { gl::DeleteShader(self.id); }
    }
}

fn shader_from_source(source: &std::ffi::CStr, kind: gl::types::GLuint) -> Result<gl::types::GLuint, String> {
    let id = unsafe { gl::CreateShader(kind) };
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }

    let mut success: gl::types::GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }

    if success == 0 {
        let mut len: gl::types::GLint = 0;
        unsafe { gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len); }
        
        let error = create_whitespace_cstring_with_len(len as usize);
        unsafe { gl::GetShaderInfoLog(id, len, std::ptr::null_mut(), error.as_ptr() as *mut gl::types::GLchar); }

        return Err(error.to_string_lossy().into_owned());
    }

    Ok(id)
}

/// Allocates a buffer of size `len`, fills it with whitespace, and converts it into a `CString` which is returned.
/// 
/// Certain OpenGL functions, namely `GetActiveUniform`, `GetProgramInfoLog`, and `GetShaderInfoLog`, require a raw 
/// pointer to a `CString` of some size that can only be determined at runtime. Moreover, the functions write to the
/// pointer, so its content must be mutable. Since memory can't be left uninitialized, the "safe" workaround is to 
/// build a buffer out of whitespace of needed length and convert it using `from_vec_unchecked`.
fn create_whitespace_cstring_with_len(len: usize) -> std::ffi::CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { std::ffi::CString::from_vec_unchecked(buffer) }
}