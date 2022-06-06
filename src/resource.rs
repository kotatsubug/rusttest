use std::io::Read;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("failed to read CString from file that contains 0")]
    FileContainsNil,

    #[error("failed to get executable path")]
    FailedToGetExePath,
}

pub struct Resource {
    root_path: std::path::PathBuf,
}

impl Resource {
    pub fn from_relative_exe_path(rel_path: &std::path::Path) -> Result<Resource, Error> {
        let exe_filename = std::env::current_exe().map_err(|_| Error::FailedToGetExePath)?;
        let exe_path = exe_filename.parent().ok_or(Error::FailedToGetExePath)?;
        
        Ok(Resource {
            root_path: exe_path.join(rel_path),
        })
    }

    pub fn from_exe_path() -> Result<Resource, Error> {
        Resource::from_relative_exe_path(std::path::Path::new(""))
    }

    pub fn load_cstring(&self, resource_name: &str) -> Result<std::ffi::CString, Error> {
        let mut file: std::fs::File = std::fs::File::open(resource_name_to_path(&self.root_path, resource_name))?;

        // Allocate buffer of the same size as FILE
        let mut buffer: Vec<u8> = Vec::with_capacity(file.metadata()?.len() as usize + 1);
        file.read_to_end(&mut buffer)?;

        // Check for nil byte
        if buffer.iter().find(|i| **i == 0).is_some() {
            return Err(Error::FileContainsNil);
        }

        Ok(unsafe { std::ffi::CString::from_vec_unchecked(buffer) })
    }
}

fn resource_name_to_path(root_dir: &std::path::Path, location: &str) -> std::path::PathBuf {
    let mut path: std::path::PathBuf = root_dir.into();

    for part in location.split("/") {
        path = path.join(part);
    }

    path
}