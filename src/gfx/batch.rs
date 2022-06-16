use crate::log::LOGGER;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("OpenGL throws error code: {}", flag)]
    OpenGLError {
        flag: u32
    },
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct f32_f32_f32 {
    pub d0: f32,
    pub d1: f32,
    pub d2: f32,
}

impl f32_f32_f32 {
    pub fn new(d0: f32, d1: f32, d2: f32) -> Self {
        f32_f32_f32{ d0, d1, d2 }
    }
}

impl From<(f32, f32, f32)> for f32_f32_f32 {
    fn from(other: (f32, f32, f32)) -> Self {
        f32_f32_f32::new(other.0, other.1, other.2)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Vertex {
    pub pos: f32_f32_f32,
    pub color: f32_f32_f32,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Mesh{
            vertices: vertices,
            indices: indices,
        }
    }
}

#[allow(dead_code)]
#[repr(C, packed)]
struct DrawArraysIndirectCmd {
    count: gl::types::GLuint,
    instance_count: gl::types::GLuint,
    first: gl::types::GLuint,
    base_instance: gl::types::GLuint,
}

#[allow(dead_code)]
#[repr(C, packed)]
struct DrawElementsIndirectCmd {
    count: gl::types::GLuint,          // # elements (i.e. indices)
    instance_count: gl::types::GLuint, // # instances (kind of like drawcalls)
    first_index: gl::types::GLuint,    // index of first element
    base_vertex: gl::types::GLint,     // indices[i] + baseVertex
    base_instance: gl::types::GLuint,  // used in calculating instance = [gl_InstanceID / divisor] + baseInstance
    
    // TODO: When getting around to compute shaders, note that GLSL layout std140 rules dictate 16-byte alignment
    // Since padding would need to be used here, glMultiDraw...Indirect commands must specify a stride of 16 bytes!
    // padding0: gl::types::GLuint,
    // padding1: gl::types::GLuint,
    // padding2: gl::types::GLuint,
}

/// Struct encapsulating all meshes, transforms, and buffers required for an OpenGL indirect multidraw call.
/// 
/// Mesh vertex and index data is decidedly immutable because its modification 
/// requires the reconstruction of all indirect draw commands. So VAO/VBO should be unchanged during its lifetime.
/// 
/// Transforms are mutable, however. Individual transforms to specific meshes in the batch 
/// are passed through as subdata into an array buffer, as all high frequency GPU data should be treated.
/// 
/// Usually for immutable vertex arrays, modern OpenGL convention says that it's better to map a buffer range
/// to a pointer and fiddle with the data that way. However, it's a very expensive operation and for a small group 
/// of meshes with individual transforms (like physics debris!), it's a lot less expensive to just use a new
/// multidraw instead of the alternative, that being mapping a buffer and then, very dangerously, manually
/// streaming new vertex data through a ring buffer, synchronizing updates when needed.
pub struct Batch {
    program_id: gl::types::GLuint,
    mesh: Mesh,

    draw_commands: Vec<DrawElementsIndirectCmd>,
    transforms: Vec<glam::Mat4>,

    vao: gl::types::GLuint,         // vertex array object
    vbo: gl::types::GLuint,         // vertex buffer object
    idxbo: gl::types::GLuint,       // index buffer object
    idbo: gl::types::GLuint,        // indirect draw buffer object
    drawidbo: gl::types::GLuint,    // draw ID buffer object
    transformbo: gl::types::GLuint, // transforms SSBO
}

impl Batch {
    pub fn new(program: gl::types::GLuint, mesh: Mesh, transforms: &Vec<glam::Mat4>) -> Result<Self, Error> {
        // TODO: probably a cleaner way, maybe by borrowing Program
        unsafe {
            gl::UseProgram(program);
        }

        let mut vao: gl::types::GLuint = 0;
        let mut vbo: gl::types::GLuint = 0;
        let mut idxbo: gl::types::GLuint = 0;
        let mut idbo: gl::types::GLuint = 0;
        let mut drawidbo: gl::types::GLuint = 0;
        let mut transformbo: gl::types::GLuint = 0;

        let mut drawids: Vec<gl::types::GLuint> = Vec::with_capacity(transforms.len());
        for i in 0..transforms.len() {
            drawids.push(i as u32);
        }

        let mut draw_commands: Vec<DrawElementsIndirectCmd> = Vec::with_capacity(transforms.len());
        for i in 0..transforms.len() {
            draw_commands.push(
                DrawElementsIndirectCmd {
                    count: mesh.indices.len() as u32,
                    instance_count: 1,
                    first_index: 0,
                    base_vertex: 0,
                    base_instance: i as u32,
                }
            );
        }

        // TODO: use DSA methods -- would be slightly faster here but
        // it would require some bindless fiddling with the array objects

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (mesh.vertices.len() * std::mem::size_of::<Vertex>()) as gl::types::GLsizeiptr,
                mesh.vertices.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );

            // Attributes of vertex buffer
            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (6 * std::mem::size_of::<f32>()) as gl::types::GLsizei,
                std::ptr::null(),
            );
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                (6 * std::mem::size_of::<f32>()) as gl::types::GLsizei,
                (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );

            gl::GenBuffers(1, &mut drawidbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, drawidbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (drawids.len() * std::mem::size_of::<gl::types::GLuint>()) as gl::types::GLsizeiptr,
                drawids.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
            // Attributes of draw ID buffer
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribIPointer(
                2,
                1,
                gl::UNSIGNED_INT,
                (std::mem::size_of::<i32>()) as gl::types::GLsizei,
                std::ptr::null(),
            );
            gl::VertexAttribDivisor(2, 1);

            gl::GenBuffers(1, &mut idxbo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, idxbo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mesh.indices.len() * std::mem::size_of::<gl::types::GLuint>()) as gl::types::GLsizeiptr,
                mesh.indices.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );

            gl::GenBuffers(1, &mut transformbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, transformbo);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, transformbo);
            gl::BufferData(
                gl::SHADER_STORAGE_BUFFER,
                (transforms.len() * 16 * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                (&transforms[0].to_cols_array()).as_ptr() as *const gl::types::GLvoid, // FIXME: does the whole Vec need .to_cols_array() ?
                gl::DYNAMIC_DRAW,
            );
            
            gl::GenBuffers(1, &mut idbo);
            gl::BindBuffer(gl::DRAW_INDIRECT_BUFFER, idbo);
            gl::BufferData(
                gl::DRAW_INDIRECT_BUFFER,
                (draw_commands.len() * std::mem::size_of::<DrawElementsIndirectCmd>()) as gl::types::GLsizeiptr,
                draw_commands.as_ptr() as *const gl::types::GLvoid,
                gl::DYNAMIC_DRAW,
            );
            
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                LOGGER().a.error(format!("OpenGL error {}", error).as_str());
            }
        }
        
        Ok(Batch {
            program_id: program,
            mesh: mesh,
            transforms: transforms.to_vec(),

            draw_commands: draw_commands,
            vao: vao,
            vbo: vbo,
            idxbo: idxbo,
            idbo: idbo,
            drawidbo: drawidbo,
            transformbo: transformbo,
        })
    }
    
    pub fn draw(&self) {
        unsafe {
            gl::UseProgram(self.program_id);
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.transformbo);
            gl::BindBuffer(gl::DRAW_INDIRECT_BUFFER, self.idbo);
            gl::MultiDrawElementsIndirect(
                gl::TRIANGLES,
                gl::UNSIGNED_INT,
                std::ptr::null(),
                self.draw_commands.len() as gl::types::GLsizei,
                0,
            );
        }
    }

    pub fn set_transform(&mut self, index: usize, transform: glam::Mat4) {
        self.transforms[index] = transform;
        unsafe {
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.transformbo);
            gl::BufferSubData(
                gl::SHADER_STORAGE_BUFFER,
                (std::mem::size_of::<f32>() * 16 * index as usize) as gl::types::GLintptr,
                (std::mem::size_of::<f32>() * 16) as gl::types::GLsizeiptr,
                (&self.transforms[index].to_cols_array()).as_ptr() as *const gl::types::GLvoid
            );
        }
    }

    pub fn set_all_transforms(&mut self, transforms: &[glam::Mat4]) {
        self.transforms = transforms.to_vec();
        unsafe {
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.transformbo);
            gl::BufferSubData(
                gl::SHADER_STORAGE_BUFFER,
                0,
                (std::mem::size_of::<f32>() * 16 * self.transforms.len()) as gl::types::GLsizeiptr,
                self.transforms.as_ptr() as *const gl::types::GLvoid
            );
        }
    }
}

impl Drop for Batch {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.idbo);
            gl::DeleteBuffers(1, &mut self.transformbo);
            gl::DeleteBuffers(1, &mut self.idxbo);
            gl::DeleteBuffers(1, &mut self.drawidbo);
            gl::DeleteBuffers(1, &mut self.vbo);
            gl::DeleteVertexArrays(1, &mut self.vao); // attributes are bound to the VAO, remove them

            // Shader program deletion done externally, other batches could be sharing it
        }
    }
}