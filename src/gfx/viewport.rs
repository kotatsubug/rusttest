
pub struct Viewport {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Viewport {
    pub fn make_viewport(width: i32, height: i32) -> Self {
        Viewport { x: 0, y: 0, width, height }
    }

    pub fn update_size(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }
    
    pub fn use_viewport(&self) {
        unsafe { gl::Viewport(self.x, self.y, self.width, self.height); }
    }
}