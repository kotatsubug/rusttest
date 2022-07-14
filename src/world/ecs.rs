use std::cell::{RefCell, RefMut};

//let data = world.borrow_component_vec::<Health>().unwrap();
//for health_component in data.iter().filter_map(|f| f.as_ref()) {
//    
//}
//let zip = world.borrow_component_vec::<Health>()
//               .unwrap()
//               .iter()
//               .zip(world.borrow_component_vec::<Name>().unwrap().iter());
//for (hp, name) in zip.filter_map(|(hp, name)| {
//    Some( (hp.as_ref()?, name.as_ref()?) )
//}) {
//    if hp < 0 {
//        println!("{} has perished...", name);
//    }
//}
//let mut healths = world.borrow_component_vec_mut::<Health>().unwrap();
//let mut names = world.borrow_component_vec_mut::<Name>().unwrap();
//let zip = healths.iter_mut().zip(names.iter_mut());
//
//for (health, name) in zip.filter_map(|(health, name)| Some((health.as_mut()?, name.as_mut()?))) {
//    if name.0 == "Perseus" && health.0 <= 0 {
//        *health = Health(100);
//    }
//}

#[derive(Debug)]
pub struct Health(pub i32);

#[derive(Debug)]
pub struct Name(pub &'static str);

/// Where all ECS data is stored
pub struct World {
    // Use `ent_count` to assign each Entity a unique ID
    pub ent_count: usize,
    // Instead of using a bunch of Vec<Option<ComponentType>> for each component,
    // use a trait to store multiple Vec<Option<ComponentType>>s all in `component_vecs`.
    // The Box is necessary because Vec requires each item to be the same size;
    // we don't know the size of each unique Vec<Option<ComponentType>>.
    pub component_vecs: Vec<Box<dyn ComponentVec>>,
}

trait ComponentVec {
    fn push_none(&mut self);
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: 'static> ComponentVec for RefCell<Vec<Option<T>>> {
    fn push_none(&mut self) {
        // `&mut self` already guarantees exclusive access to Self,
        // so `get_mut` can be used here to avoid runtime checks.
        self.get_mut().push(None)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            ent_count: 0,
            component_vecs: Vec::new(),
        }
    }

    /// Create a new entity and add it to the world. Returns unique entity ID.
    pub fn new_entity(&mut self) -> usize {
        let ent_id = self.ent_count;
        for component_vec in self.component_vecs.iter_mut() {
            component_vec.push_none(); // initialized with no components
        }
        self.ent_count += 1;

        ent_id
    }

    pub fn add_component_to_entity<ComponentType: 'static>(&mut self, entity: usize, component: ComponentType) {
        // First, iterate through all `component_vecs` to find a matching one.
        // On success, insert the component and return.
        for component_vec in self.component_vecs.iter_mut() {
            if let Some(component_vec) = component_vec.as_any_mut()
                                                      .downcast_mut::<RefCell<Vec<Option<ComponentType>>>>()
            {
                component_vec.get_mut()[entity] = Some(component);
                return;
            }
        }

        // Otherwise, no matching component storage exists. One must be made!
        let mut new_component_vec: Vec<Option<ComponentType>> = Vec::with_capacity(self.ent_count);
        // All existing entities don't have this component, so give them `None`
        for _ in 0..self.ent_count {
            new_component_vec.push(None);
        }
        // Give this entity the component.
        new_component_vec[entity] = Some(component);
        self.component_vecs.push(Box::new(RefCell::new(new_component_vec)));
    }

    /// Finds and borrows the `ComponentVec` that matches a type (if it exists).
    pub fn borrow_component_vec_mut<ComponentType: 'static>(&self) -> Option<RefMut<Vec<Option<ComponentType>>>> {
        for component_vec in self.component_vecs.iter() {
            if let Some(component_vec) = component_vec.as_any()
                                                      .downcast_ref::<RefCell<Vec<Option<ComponentType>>>>()
            {
                return Some(component_vec.borrow_mut());
            }
        }

        None
    }
}