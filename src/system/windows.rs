
use std::iter::once;
use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_SYSTEMMODAL};

#[derive(Debug, Copy, Clone)]
pub enum IconType {
    Error,
    Info,
    None,
}

impl std::fmt::Display for IconType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MsgBoxError {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    #[error("failed to create a message box!")]
    Create(()),
}

#[cfg(target_os = "windows")]
pub fn create_message_box(title: &str, content: &str, icon_type: IconType) -> Result<(), MsgBoxError> {
    let lp_caption: Vec<u16> = title.encode_utf16().chain(once(0)).collect();
    let lp_text: Vec<u16> = content.encode_utf16().chain(once(0)).collect();
    
    let window_type = match icon_type {
        IconType::Error => { MB_OK | MB_ICONERROR | MB_SYSTEMMODAL },
        IconType::Info =>  { MB_OK | MB_ICONINFORMATION | MB_SYSTEMMODAL },
        IconType::None =>  { MB_OK | MB_SYSTEMMODAL },
    };

    unsafe {
        match MessageBoxW(std::ptr::null_mut(), lp_text.as_ptr(), lp_caption.as_ptr(), window_type) {
            0 => Err(MsgBoxError::Create(())),
            _ => Ok(()),
        }
    }
}