use crate::math::isometry::TransformEuler;

pub struct Camera {
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
    pub transform: TransformEuler,
    // TODO: specific program variable for rendering?

    /// 3D camera vectors used for calculating the current 
    /// view matrix in conjunction with camera rotation.
    /// ```
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
    /// ```
    front: glam::Vec3,
    right: glam::Vec3,
    up: glam::Vec3,
    worldup: glam::Vec3,
}

impl Camera {
    pub fn new(
        view_: glam::Mat4,
        projection_: glam::Mat4,
        transform_: TransformEuler,
        worldup_: glam::Vec3
    ) -> Self {
        let updated_vec = glam::vec3(
            f32::cos(transform_.euler_rotation.y) * f32::cos(transform_.euler_rotation.x),
            f32::sin(transform_.euler_rotation.x),
            f32::sin(transform_.euler_rotation.y) * f32::cos(transform_.euler_rotation.x),
        );
        let front_ = glam::Vec3::normalize(updated_vec);
        let right_ = glam::Vec3::normalize(front_.cross(worldup_));
        let up_ = glam::Vec3::normalize(right_.cross(front_));

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
        let updated_vec = glam::vec3(
            f32::cos(self.transform.euler_rotation.y) * f32::cos(self.transform.euler_rotation.x),
            f32::sin(self.transform.euler_rotation.x),
            f32::sin(self.transform.euler_rotation.y) * f32::cos(self.transform.euler_rotation.x),
        ); // direction the camera is currently facing, unnormalized
        self.front = glam::Vec3::normalize(updated_vec);
        self.right = glam::Vec3::normalize(self.front.cross(self.worldup));
        self.up = glam::Vec3::normalize(self.right.cross(self.front));
    }

    pub fn translate_forward(&mut self, dist: f32) {
        self.transform.position += self.front * dist;
    }

    pub fn translate_left(&mut self, dist: f32) {
        self.transform.position += self.right * dist;
    }

    pub fn translate_up(&mut self, dist: f32) {
        self.transform.position += self.up * dist;
    }

    /// Adds an euler rotation to current transform rotation.
    /// This should be used instead of accessing `transform.euler_rotation` because it also prevents overflow.
    pub fn rotate(&mut self, euler: glam::Vec3) {
        self.transform.euler_rotation += euler;
        
        // Constrain pitch to (-π/2, π/2)
        // ε needed to remove weirdness since `front` can be flipped at ±π/2 pitch
        if self.transform.euler_rotation.x > std::f32::consts::PI / 2.0 - f32::EPSILON {
            self.transform.euler_rotation.x = std::f32::consts::PI / 2.0 - f32::EPSILON;
        } else if self.transform.euler_rotation.x < -std::f32::consts::PI / 2.0 + f32::EPSILON {
            self.transform.euler_rotation.x = -std::f32::consts::PI / 2.0 + f32::EPSILON;
        }
        
        // Smooth wrap current yaw to [0, 2π)
        // Use fmodulus trick so we can support rotations greater than 2π without snapping to 0 in constant time
        self.transform.euler_rotation.y = 
            (std::f32::consts::PI * 2.0 + 
                (self.transform.euler_rotation.y % (std::f32::consts::PI * 2.0))
            ) % (std::f32::consts::PI * 2.0);
    }
}