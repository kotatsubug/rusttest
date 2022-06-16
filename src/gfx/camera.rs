use crate::math::affine::AffineTransform;

pub struct Camera {
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
    pub transform: AffineTransform,
    // program?

    /// 3D camera vectors used for calculating the current 
    /// view matrix in conjunction with camera rotation
    ///                                                 
    ///             ^              ^                    
    ///          up |              | worldup (immutable)
    ///          ___|______        |                    
    ///         /___|____/ |       |                    
    ///  right |    |    | |       |                    
    /// <------|--((O))  | |                            
    ///        |___/_____|/                             
    ///           /                                     
    ///          / front                                
    ///         v                                       
    ///                                                 
    front: glam::Vec3,
    right: glam::Vec3,
    up: glam::Vec3,
    worldup: glam::Vec3,
}

impl Camera {
    pub fn new(
        view_: glam::Mat4,
        projection_: glam::Mat4,
        transform_: AffineTransform,
        worldup_: glam::Vec3
    ) -> Self {
        
        // TODO: generalize this and update fn
        let up = glam::const_vec3!([ 0.0, 0.0, 1.0 ]);
        // TODO: faster to rotate the axes instead of cross product?
        let front_ = (transform_.rotation.mul_vec3(up)).normalize();
        let right_ = (front_.cross(worldup_)).normalize();
        let up_ = (right_.cross(front_)).normalize();

        Camera {
            view: view_,
            projection: projection_,
            transform: transform_,
            front: front_,
            right: right_,
            up: up_,
            worldup: worldup_,
        }
    }
    
    /// Update camera's view matrix. Then, update camera's front-right-up vectors.
    pub fn update_view(&mut self) {
        let target = self.transform.position + self.front;
        self.view = glam::Mat4::look_at_lh(self.transform.position, target, self.up);
        self.update_camera_vectors();
    }

    fn update_camera_vectors(&mut self) {
        // Vector to rotate into place for getting the camera front (normal)
        // (0,0,1) for LH (OpenGL), (0,0,-1) for RH coordinate systems.
        let no_angle = glam::const_vec3!([ 0.0, 0.0, 1.0 ]);
        // TODO: faster to rotate the axes instead of cross product?
        self.front = (self.transform.rotation.mul_vec3(no_angle)).normalize();
        self.right = (self.front.cross(self.worldup)).normalize();
        self.up = (self.right.cross(self.front)).normalize();
    }
}