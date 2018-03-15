use gl;
use std;
use std::collections::HashMap;
use std::ffi::{CString, CStr};
use resources::{self, Resources};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Failed to load resource {}", name)]
    ResourceLoad { name: String, #[cause] inner: resources::Error },
    #[fail(display = "Can not determine shader type for resource {}", name)]
    CanNotDetermineShaderTypeForResource { name: String },
    #[fail(display = "Failed to compile shader {}: {}", name, message)]
    CompileError { name: String, message: String },
    #[fail(display = "Failed to link program {}: {}", name, message)]
    LinkError { name: String, message: String },
}

struct Uniform {
    id: gl::types::GLint, 
    typ: gl::types::GLenum,
}
pub struct Program {
    gl: gl::Gl,
    id: gl::types::GLuint,

    uniforms: HashMap<String, Uniform>,
}

impl Program {
    pub fn from_res(gl: &gl::Gl, res: &Resources, name: &str) -> Result<Program, Error> {
        const POSSIBLE_EXT: [&str; 2] = [
            ".vert",
            ".frag",
        ];

        let resource_names = POSSIBLE_EXT.iter()
            .map(|file_extension| format!("{}{}", name, file_extension))
            .collect::<Vec<String>>();

        let shaders = resource_names.iter()
            .map(|resource_name| {
                Shader::from_res(gl, res, resource_name)
            })
            .collect::<Result<Vec<Shader>, Error>>()?;

        Program::from_shaders(gl, &shaders[..])
            .map_err(|message| Error::LinkError { name: name.into(), message })
    }

    pub fn from_shaders(gl: &gl::Gl, shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl.CreateProgram() };

        for shader in shaders {
            unsafe { gl.AttachShader(program_id, shader.id()); }
        }

        unsafe { gl.LinkProgram(program_id); }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl.GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe {
                gl.GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl.GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar
                );
            }

            return Err(error.to_string_lossy().into_owned());
        }

        for shader in shaders {
            unsafe { gl.DetachShader(program_id, shader.id()); }
        }

        let uniforms = Program::get_uniforms(gl, program_id);
        Ok(Program { gl: gl.clone(), id: program_id, uniforms: uniforms})
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }

    pub fn set_used(&self) {
        unsafe {
            self.gl.UseProgram(self.id);
        }
    }

    fn get_uniforms(gl: &gl::Gl, id: gl::types::GLuint) -> HashMap<String, Uniform> {

        let mut uniforms: HashMap<String, Uniform>;
        uniforms = HashMap::new();

        let mut total: gl::types::GLint = -1;
        unsafe {
            gl.GetProgramiv(id, gl::ACTIVE_UNIFORMS, 
                                 &mut total as *mut gl::types::GLint);
        }
        for u in 0..total {
            let mut name_len: i32 = -1;
            let mut num: i32 = -1;
            let mut typ : gl::types::GLenum = gl::ZERO;
            let name = create_whitespace_cstring_with_len(256 as usize);

            unsafe {
                gl.GetActiveUniform(id, u as u32, 255, 
                    &mut name_len as *mut gl::types::GLint, 
                    &mut num as *mut gl::types::GLint, 
                    &mut typ as *mut gl::types::GLenum, 
                    name.as_ptr() as *mut gl::types::GLchar);
            }
            let loc : gl::types::GLint;
            unsafe {
                loc = gl.GetUniformLocation(id, name.as_ptr());
            }
            let name_slice : &str = name.to_str().unwrap();
            let name_str : String = name_slice.to_owned();

            let uniform = Uniform{id: loc, typ: typ,};
            uniforms.insert(name_str, uniform);
        }
        uniforms
    }

    pub fn uniform_loc(&self, name: &CStr) -> gl::types::GLint {
        unsafe {
            self.gl.GetUniformLocation(self.id, name.as_ptr())
        }
    }

    pub fn set_uniform1f(&self, name: String, value: gl::types::GLfloat) -> Result<(),String> {
        let uniform = &self.uniforms[&name];
        if uniform.typ == gl::FLOAT {
            unsafe {
                self.gl.Uniform1f(uniform.id, value);
            }
        } else {
            Err("This uniform takes a float");
        }
        Ok(())
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteProgram(self.id);
        }
    }
}

pub struct Shader {
    gl: gl::Gl,
    id: gl::types::GLuint,
}

impl Shader {
    pub fn from_res(gl: &gl::Gl, res: &Resources, name: &str) -> Result<Shader, Error> {
        const POSSIBLE_EXT: [(&str, gl::types::GLenum); 2] = [
            (".vert", gl::VERTEX_SHADER),
            (".frag", gl::FRAGMENT_SHADER),
        ];

        let shader_kind = POSSIBLE_EXT.iter()
            .find(|&&(file_extension, _)| {
                name.ends_with(file_extension)
            })
            .map(|&(_, kind)| kind)
            .ok_or_else(|| Error::CanNotDetermineShaderTypeForResource { name: name.into() })?;

        let source = res.load_cstring(name)
            .map_err(|e| Error::ResourceLoad { name: name.into(), inner: e })?;

        Shader::from_source(gl, &source, shader_kind)
            .map_err(|message| Error::CompileError { name: name.into(), message })
    }

    pub fn from_source(
        gl: &gl::Gl,
        source: &CStr,
        kind: gl::types::GLenum
    ) -> Result<Shader, String> {
        let id = shader_from_source(gl, source, kind)?;
        Ok(Shader { gl: gl.clone(), id })
    }

    pub fn from_vert_source(gl: &gl::Gl, source: &CStr) -> Result<Shader, String> {
        Shader::from_source(gl, source, gl::VERTEX_SHADER)
    }

    pub fn from_frag_source(gl: &gl::Gl, source: &CStr) -> Result<Shader, String> {
        Shader::from_source(gl, source, gl::FRAGMENT_SHADER)
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteShader(self.id);
        }
    }
}

fn shader_from_source(
    gl: &gl::Gl,
    source: &CStr,
    kind: gl::types::GLenum
) -> Result<gl::types::GLuint, String> {
    let id = unsafe { gl.CreateShader(kind) };
    unsafe {
        gl.ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl.CompileShader(id);
    }

    let mut success: gl::types::GLint = 1;
    unsafe {
        gl.GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }

    if success == 0 {
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl.GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        }

        let error = create_whitespace_cstring_with_len(len as usize);

        unsafe {
            gl.GetShaderInfoLog(
                id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar
            );
        }

        return Err(error.to_string_lossy().into_owned());
    }

    Ok(id)
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.extend([b' '].iter().cycle().take(len));
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}
