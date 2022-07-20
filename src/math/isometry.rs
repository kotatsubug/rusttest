use std::rc::{Rc, Weak};
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct Transform3 {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
    // The link from child to parent must be downgraded from `Rc` to `Weak` to avoid `Rc<RefCell>` circular references.
    // There are other ways of doing this, but `RefCell`s provide easier mutability.
    //parent: Weak<RefCell<AffineTransform>>,
    //children: Vec<Rc<RefCell<AffineTransform>>>
}

#[derive(Debug, Clone)]
pub struct TransformEuler {
    pub position: glam::Vec3,
    /// In the form of ```(pitch, yaw, roll)```.
    pub euler_rotation: glam::Vec3,
}

impl Transform3 {
    pub fn new(position: glam::Vec3, rotation: glam::Quat, scale: glam::Vec3) -> Self {
        Transform3 {
            position: position,
            rotation: rotation,
            scale: scale,

            //parent: Weak::new(),
            //children: Vec::new(),
        }
    }

    /// Multiply this `Transform3`'s rotation by a quaternion and then normalize the result.
    pub fn rotate(&mut self, other: glam::Quat) {
        self.rotation = self.rotation.mul_quat(other).normalize();
    }

    // Adds `Self` as a child of `parent`, then sets parent of `Self` to `target`.
    // If a parent already exists, removes `Self` from its children. This overwrites the current parent.
    //pub fn parent_to(&mut self, target: &mut AffineTransform) {
    //    if self.parent.weak_count() > 0 {
    //        self.parent = Weak::new();
    //    }
    //    target.add_child(self);
    //    // need to avoid circular references but keep mutability, so downgrade RefCell Rc
    //    self.parent = Rc::downgrade(&Rc::new(RefCell::new(target)));
    //}

    //fn add_child(&mut self, child: &mut AffineTransform) {
    //    self.children.push(
    //        Rc::new(RefCell::new(child))
    //    );
    //}
}

impl Drop for Transform3 {
    fn drop(&mut self) {

    }
}

impl TransformEuler {
    pub fn new(position: glam::Vec3, rotation: glam::Vec3) -> Self {
        TransformEuler {
            position: position,
            euler_rotation: rotation,
        }
    }
}

impl Drop for TransformEuler {
    fn drop(&mut self) {

    }
}