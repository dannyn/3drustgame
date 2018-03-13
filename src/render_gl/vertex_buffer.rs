use std;

extern crate sdl2;
extern crate gl;

pub struct VertexBuffer {
    gl: gl::Gl,
    glid: gl::types::GLuint,
}

impl VertexBuffer {
    pub fn new_vertex_buffer(gl : &gl::Gl, vertices: Vec<f32>) -> VertexBuffer {

        let mut vbo = VertexBuffer{
            gl: gl.clone(),
            glid: 0,
        };

        unsafe {
            gl.GenBuffers(1, &mut vbo.glid);
            gl.BindBuffer(gl::ARRAY_BUFFER, vbo.glid);
            gl.BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                vertices.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
            gl.BindBuffer(gl::ARRAY_BUFFER, 0);

        }

        vbo
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.BindBuffer(gl::ARRAY_BUFFER, self.glid);
        }
    }
}

