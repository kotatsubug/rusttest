
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("OpenGL throws error code: {}", flag)]
    OpenGLError {
        flag: u32
    },
}

#[repr(C, packed)]
struct DrawArraysIndirectCmd {
    count: gl::types::GLuint,
    instance_count: gl::types::GLuint,
    first: gl::types::GLuint,
    base_instance: gl::types::GLuint,
}

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
/// 
pub struct Batch {
    program_id: gl::types::GLuint,
    // mesh: Mesh,

    draw_commands: DrawElementsIndirectCmd,
    // transforms: Vec<Transform>

    vao: gl::types::GLuint,         // vertex array object
    vbo: gl::types::GLuint,         // vertex buffer object
    idxbo: gl::types::GLuint,       // index buffer object
    idbo: gl::types::GLuint,        // indirect draw buffer object
    drawidbo: gl::types::GLuint,    // draw ID buffer object
    transformbo: gl::types::GLuint, // transforms SSBO
}

impl Batch {
    pub fn make_batch(program: gl::types::GLuint, mesh: Mesh, transforms: Vec<Transform>) -> Result<Batch, Error> {
        unsafe { gl::UseProgram(program); } // TODO: clean this up

        // Fill up draw commands, draw IDs, ...

        // ...

        // TODO: use DSA methods -- would be slightly faster here but
        // it would require some bindless fiddling with the array objects

        let mut vao: gl::types::GLuint = 0;
        let mut vbo: gl::types::GLuint = 0;
        let mut idxbo: gl::types::GLuint = 0;
        let mut idbo: gl::types::GLuint = 0;
        let mut drawidbo: gl::types::GLuint = 0;
        let mut transformbo: gl::types::GLuint = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<Vert>()) as gl::types::GLsizeiptr,
                vertices.as_ptr() as *const gl::types::GLvoid,
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
                (6 * std::mem::size_of::<f32>()) as gl::types::GLsizei,  // NOTE:  May need to be 3, not 6.
                std::ptr::null(),
            );
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                (6 * std::mem::size_of::<f32>()) as gl::types::GLsizei,  // NOTE:  May need to be 3, not 6.
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
                (indices.len() * std::mem::size_of::<gl::types::GLuint>()) as gl::types::GLsizeiptr,
                indices.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );

            gl::GenBuffers(1, &mut transformbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, transformbo);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, transformbo);
            gl::BufferData(
                gl::SHADER_STORAGE_BUFFER,
                (transforms.len() * 16 * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                transforms[0].as_ptr() as *const gl::types::GLvoid,
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

            // TODO: This should be logged instead!
            //let error = gl::GetError();
            //if error != gl::NO_ERROR {
            //    return Err(Error::OpenGLError{ flag: error });
            //}
        }
        
        Ok(Batch {
            program_id: program,
            mesh: mesh,
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
            gl::UseProgram(program_id);
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, transformbo);
            gl::BindBuffer(gl::DRAW_INDIRECT_BUFFER, idbo);
            gl::MultiDrawElementsIndirect(
                gl::TRIANGLES,
                gl::UNSIGNED_INT,
                (GLvoid*)0, draw_commands.size(),
                0,
            );
        }
    }

    pub fn set_transform(&self, index: u32, transform: &NDArray<f32, 2>) {
        unsafe {
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, transformbo);
            gl::BufferSubData(
                gl::SHADER_STORAGE_BUFFER,
                sizeof(float)*16*index,
                sizeof(float)*16,
                &*transform_in.begin()
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
            gl::DeleteVertexArrays(1, &mut self.vao); // Attributes are bound to the VAO
            // Shader program deletion done externally, other batches could be sharing it
        }
    }
}