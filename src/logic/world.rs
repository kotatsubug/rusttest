//! The ECS hierarchy used can be described by
//! 
//! ```
//! World
//! ├ // * Various entity metadata ...
//! └ Vec<Archetype>
//!       ├ components: Vec<ComponentStore>
//!       │                 ├ TypeId
//!       │                 └ Boxed ComponentColumn
//!       │                   (can be downcast into a RwLock<Vec<T>>)
//!       └ entities: Vec<Entity>
//!                       └ u64
//! ```
//! 
//! `ComponentColumn` is just a `Vec` of component traits. The diagram below should make it easier to understand
//! why I choose to call it a column.
//! 
//! This is largely a traditional data-oriented designed ECS, but with archetypes.
//! Entities sharing identical components belong to the same `Archetype`.
//! 
//! This drastically improves performance of queries over components, since the iteration is done at the archetype 
//! level and not the component level. However, it's at the cost of O(**n**) entity initialization instead of O(**1**) 
//! for **n** components. This is perfect for my use case.
//! 
//! ## Non-archetypal ECS
//! ```
//!         world.create_entity()
//!             .with(Position)─┐
//!             .with(Sprite);──│─────┐
//!                             │     │
//!         Positions   Velocity│ Sprite
//!         column      column  │ column
//!         ┌──────────┬────────│─┬───▼────┐
//!         │┌────────┐│        │ │┌──────┐│
//! Entity 0││Position│◄────────┘ ││Sprite││
//!         │└────────┘│          │└──────┘│
//!         │┌────────┐│┌────────┐│        │
//! Entity 1││Position│││Velocity││        │
//!         │└────────┘│└────────┘│        │
//!         │┌────────┐│┌────────┐│        │
//! Entity 2││Position│││Velocity││        │
//!         │└────────┘│└────────┘│        │
//!         └──────────┴──────────┴────────┘
//! ```
//! ## Archetypal ECS
//! ```
//!         world.insert_entity(
//!             (Position, Sprite)
//!         );    │          │
//!               │          │
//!         Archetype "PS"   │ Archetype "PV"
//!         ┌─────▼──────────▼─┬────────────────────┐
//!         │┌────────┐┌──────┐│                    │
//! Entity 0││Position││Sprite││                    │
//!         │└────────┘└──────┘│                    │
//!         │                  │┌────────┐┌────────┐│
//! Entity 1│                  ││Position││Velocity││
//!         │                  │└────────┘└────────┘│
//!         │                  │┌────────┐┌────────┐│
//! Entity 2│                  ││Position││Velocity││
//!         │                  │└────────┘└────────┘│
//!         └──────────────────┴────────────────────┘
//! ```

use std::any::{Any, TypeId};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

use super::query::*;
use super::error::*;

pub type EntityId = u64;

/// See diagram. A trait of components belonging to an archetype column.
trait ComponentColumn: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn len(&mut self) -> usize;
    fn swap_remove(&mut self, index: EntityId);
    fn migrate(&mut self, entity_index: EntityId, other_archetype: &mut dyn ComponentColumn);
    fn new_empty_column(&self) -> Box<dyn ComponentColumn + Send + Sync>;
}

impl<T: Sync + Send + 'static> ComponentColumn for RwLock<Vec<T>> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn len(&mut self) -> usize {
        // TODO: unnecessary overhead to call read?
        self.get_mut().unwrap().len()
    }

    fn swap_remove(&mut self, index: EntityId) {
        self.get_mut().unwrap().swap_remove(index as usize);
    }

    fn migrate(&mut self, entity_index: EntityId, other_component_column: &mut dyn ComponentColumn) {
        let data: T = self.get_mut().unwrap().swap_remove(entity_index as usize);
        component_column_to_mut(other_component_column).push(data);
    }

    fn new_empty_column(&self) -> Box<dyn ComponentColumn + Send + Sync> {
        Box::new(RwLock::new(Vec::<T>::new()))
    }
}

/// TODO: This can be made unchecked in the future iif there's confidence in everything else.
fn component_column_to_mut<T: 'static>(c: &mut dyn ComponentColumn) -> &mut Vec<T> {
    c.as_any_mut()
     .downcast_mut::<RwLock<Vec<T>>>()
     .unwrap()
     .get_mut()
     .unwrap()
}

pub struct ComponentStore {
    pub type_id: TypeId,
    data: Box<dyn ComponentColumn + Send + Sync>,
}

impl ComponentStore {
    pub fn new<T: 'static + Send + Sync>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            data: Box::new(RwLock::new(Vec::<T>::new())),
        }
    }

    /// Create a new `ComponentStore` with same internal storage type as `Self`.
    pub fn new_same_type(&self) -> Self {
        Self {
            type_id: self.type_id,
            data: self.data.new_empty_column(),
        }
    }
}

pub struct Archetype {
    /// List of entities.
    pub entities: Vec<EntityId>,
    /// A collection of `ComponentStore`, which is an abstracted away `Box<dyn ComponentColumn>` 
    /// with thread boundary transfer/sharing and an associated `TypeId`.
    pub components: Vec<ComponentStore>,
}

impl Archetype {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn get<T: 'static>(&self, index: usize) -> &RwLock<Vec<T>> {
        self.components[index]
            .data
            .as_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
    }

    pub fn remove_entity(&mut self, index: EntityId) -> EntityId {
        for c in self.components.iter_mut() {
            c.data.swap_remove(index)
        }

        let moved = *self.entities.last().unwrap();
        self.entities.swap_remove(index as usize);

        moved
    }

    pub fn mutable_component_store<T: 'static>(&mut self, component_index: usize) -> &mut Vec<T> {
        component_column_to_mut(&mut *self.components[component_index].data)
    }

    pub fn replace_component<T: 'static>(&mut self, component_index: usize, index: EntityId, t: T) {
        self.mutable_component_store(component_index)[index as usize] = t;
    }

    pub fn push<T: 'static>(&mut self, component_index: usize, t: T) {
        self.mutable_component_store(component_index).push(t)
    }

    pub fn get_component_mut<T: 'static>(&mut self, index: EntityId) -> Result<&mut T, EntityMissingComponent> {
        let type_id = TypeId::of::<T>();
        let mut component_index = None;

        for (i, c) in self.components.iter().enumerate() {
            if c.type_id == type_id {
                component_index = Some(i);
                break;
            }
        }

        if let Some(component_index) = component_index {
            Ok(&mut self.mutable_component_store(component_index)[index as usize])
        } else {
            Err(EntityMissingComponent::new::<T>(index))
        }
    }

    /// Removes the component from an entity and pushes it to the `other_archetype`.
    /// The type does not need to be known to call this, but the types of `component_index` and `other_index` 
    /// must match.
    pub fn migrate_component(&mut self, component_index: usize, entity_index: EntityId, other_archetype: &mut Archetype, other_index: usize) {
        self.components[component_index].data.migrate(entity_index, &mut *other_archetype.components[other_index].data);
    }

    /// This takes a mutable reference so that the inner `RwLock` does not need to be locked 
    /// (by instead using `get_mut`).
    pub fn len(&mut self) -> usize {
        self.entities.len()
    }
}

/// Entity location in `World`.
#[derive(Debug, Clone, Copy)]
pub struct EntityLocation {
    archetype_index: EntityId,
    index_in_archetype: EntityId,
}

#[derive(Clone, Copy)]
pub struct EntityInfo {
    pub generation: EntityId,
    pub location: EntityLocation,
}

/// Handle to an `Entity` in `World`.
#[derive(Debug, Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity {
    pub index: EntityId,
    pub generation: EntityId,
}

/// Holds all components and associates entities.
pub struct World {
    pub archetypes: Vec<Archetype>,
    bundle_id_to_archetype: HashMap<u64, usize>,
    pub entities: Vec<EntityInfo>,
    free_entities: Vec<EntityId>,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            bundle_id_to_archetype: HashMap::new(),
            entities: Vec::new(),
            free_entities: Vec::new(),
        }
    }

    /// Spawn an entity with components passed as tuple.
    /// ## Example
    /// ```
    /// let mut world = World::new();
    /// let entity = world.spawn((Name("Matsumoto"), Health(100)));
    /// ```
    pub fn spawn(&mut self, b: impl ComponentBundle) -> Entity {
        let (index, generation) = if let Some(index) = self.free_entities.pop() {
            let (generation, _) = self.entities[index as usize].generation.overflowing_add(1);

            (index, generation)
        } else {
            // Push placeholder data
            self.entities.push(EntityInfo {
                generation: 0,
                location: EntityLocation {
                    archetype_index: 0,
                    index_in_archetype: 0,
                }
            });

            // Error if too many entities allocated
            debug_assert!(self.entities.len() <= EntityId::MAX as usize);
            
            ((self.entities.len() - 1) as EntityId, 0)
        };

        let location = b.spawn_in_world(self, index);

        self.entities[index as usize] = EntityInfo {
            generation: generation,
            location: location,
        };

        Entity {
            index: index,
            generation: generation,
        }
    }

    /// Spawn entity with only a single component.
    pub fn spawn_single<T: Sync + Send + 'static>(&mut self, t: T) -> Entity {
        self.spawn( (t,) )
    }

    /// Remove an entity and all of its components from the world. Error if entity does not exist.
    pub fn despawn(&mut self, entity: Entity) -> Result<(), NoSuchEntity> {
        // Remove an entity, update swapped entity position if an entity was moved
        let entity_info = self.entities[entity.index as usize];
        if entity_info.generation == entity.generation {
            self.entities[entity.index as usize].generation += 1;
            let moved_entity = self.archetypes[entity_info.location.archetype_index as usize]
                               .remove_entity(entity_info.location.index_in_archetype);
            self.free_entities.push(entity.index);

            // Update position of an entity that was moved
            self.entities[moved_entity as usize].location = entity_info.location;

            Ok(())
        } else {
            Err(NoSuchEntity)
        }
    }

    /// Get mutable access to a single component on an `Entity`.
    pub fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Result<&mut T, ComponentError> {
        let entity_info = self.entities[entity.index as usize];
        if entity_info.generation == entity.generation {
            let archetype = &mut self.archetypes[entity_info.location.archetype_index as usize];

            archetype.get_component_mut(entity_info.location.index_in_archetype)
                     .map_err(|e| ComponentError::EntityMissingComponent(e))
        } else {
            Err(ComponentError::NoSuchEntity(NoSuchEntity))
        }
    }

    /// Add a component to an entity. If the component already exists, its data will be replaced. Expensive.
    pub fn add_component<T: 'static + Send + Sync>(&mut self, entity: Entity,  t: T) -> Result<(), NoSuchEntity> {
        // When a component is added the entity can be either migrated to 
        // - a brand new archetype, or
        // - an existing archetype.
        // So, first, find if the entity exists
        let entity_info = self.entities[entity.index as usize];
        if entity_info.generation == entity.generation {
            let type_id = TypeId::of::<T>();

            // First, check if the component already exists for this entity
            let current_archetype = &self.archetypes[entity_info.location.archetype_index as usize];

            let mut type_ids: Vec<TypeId> = current_archetype.components
                                                             .iter()
                                                             .map(|c| c.type_id)
                                                             .collect();
            let binary_search_index = type_ids.binary_search(&type_id);

            if let Ok(insert_index) = binary_search_index {
                // Component already exists, replace it
                let current_archetype = &mut self.archetypes[entity_info.location.archetype_index as usize];
                current_archetype.replace_component(insert_index, entity_info.location.index_in_archetype, t);
            } else {
                // The component does not already exist in the current archetype.
                // Find an existing archetype to migrate to or create a new archetype

                let insert_index = binary_search_index.unwrap_or_else(|i| i);

                type_ids.insert(insert_index, type_id);
                let bundle_id = calculate_bundle_id(&type_ids);

                let new_archetype_index = if let Some(new_archetype_index) = self.bundle_id_to_archetype.get(&bundle_id) {
                    // Found an existing archetype to migrate data to
                    *new_archetype_index
                } else {
                    // Create a new archetype with the structure of the current archetype and one additional component
                    let mut archetype = Archetype::new();
                    for c in current_archetype.components.iter() {
                        archetype.components.push(c.new_same_type());
                    }

                    let new_archetype_index = self.archetypes.len();
                    archetype.components.insert(insert_index, ComponentStore::new::<T>());
                    self.bundle_id_to_archetype.insert(bundle_id, new_archetype_index);

                    self.archetypes.push(archetype);

                    new_archetype_index
                };

                // `index_twice` lets us mutably borrow from the world twice
                let (old_archetype, new_archetype) = index_twice(
                    &mut self.archetypes,
                    entity_info.location.archetype_index as usize,
                    new_archetype_index,
                );

                // If an entity is being moved, update its location
                if let Some(last) = old_archetype.entities.last() {
                    self.entities[*last as usize].location = entity_info.location;
                }

                // First, update the entity's location to reflect the changes about to be made...
                self.entities[entity.index as usize].location = EntityLocation {
                    archetype_index: new_archetype_index as EntityId,
                    index_in_archetype: (new_archetype.len()) as EntityId,
                };

                // ...the new archetype is the same as the old one but with one additional component...
                for i in 0..insert_index {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i,
                    );
                }

                // ...push the new component to the new archetype!
                new_archetype.push(insert_index, t);

                let components_in_archetype = old_archetype.components.len();

                for i in insert_index..components_in_archetype {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i + 1,
                    );
                }

                old_archetype.entities.swap_remove(entity_info.location.index_in_archetype as usize);
                new_archetype.entities.push(entity.index);
            }

            Ok(())
        } else {
            Err(NoSuchEntity)
        }
    }

    /// Remove a single component from an entity. If successful, removed component is returned.
    /// ## Example
    /// ```
    /// let entity = world.spawn((Name("Matsumoto"), Health(100)));
    /// let b = world.remove_component::<Health>(entity).unwrap();
    /// ```
    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Result<T, ComponentError> {
        let entity_info = self.entities[entity.index as usize];

        if entity_info.generation == entity.generation {
            let current_archetype = &self.archetypes[entity_info.location.archetype_index as usize];

            let type_id = TypeId::of::<T>();
            let mut type_ids: Vec<TypeId> = current_archetype.components
                                                             .iter()
                                                             .map(|c| c.type_id)
                                                             .collect();
            let binary_search_index = type_ids.binary_search(&type_id);

            if let Ok(remove_index) = binary_search_index {
                type_ids.remove(remove_index);
                let bundle_id = calculate_bundle_id(&type_ids);
                let new_archetype_index = if let Some(new_archetype_index) = self.bundle_id_to_archetype.get(&bundle_id) {
                    *new_archetype_index
                } else {
                    // Create a new archetype
                    let mut archetype = Archetype::new();
                    for c in current_archetype.components.iter() {
                        if c.type_id != type_id {
                            archetype.components.push(c.new_same_type());
                        }
                    }

                    let new_archetype_index = self.archetypes.len();

                    self.bundle_id_to_archetype.insert(bundle_id, new_archetype_index);
                    self.archetypes.push(archetype);
                    new_archetype_index
                };

                // `index_twice` lets us mutably borrow from the world twice
                let (old_archetype, new_archetype) = index_twice(
                    &mut self.archetypes,
                    entity_info.location.archetype_index as usize,
                    new_archetype_index,
                );

                // If an entity is being moved, update its location
                if let Some(last) = old_archetype.entities.last() {
                    self.entities[*last as usize].location = entity_info.location;
                }

                // First, update the entity's location to reflect the changes about to be made...
                self.entities[entity.index as usize].location = EntityLocation {
                    archetype_index: new_archetype_index as EntityId,
                    index_in_archetype: (new_archetype.len()) as EntityId,
                };

                // ...the new archetype is the same as the old one but with one fewer components!
                for i in 0..remove_index {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i,
                    );
                }

                let components_in_archetype = old_archetype.components.len();

                for i in (remove_index + 1)..components_in_archetype {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i - 1,
                    );
                }

                old_archetype.entities.swap_remove(entity_info.location.index_in_archetype as usize);
                new_archetype.entities.push(entity.index);

                Ok(
                    component_column_to_mut::<T>(&mut *old_archetype.components[remove_index].data)
                        .swap_remove(entity_info.location.index_in_archetype as usize),
                )
            } else {
                // Component is not in entity
                Err(ComponentError::EntityMissingComponent(
                    EntityMissingComponent::new::<T>(entity.index),
                ))
            }
        } else {
            // Entity is not in world
            Err(ComponentError::NoSuchEntity(NoSuchEntity))
        }
    }

     /// Query for an *immutable* reference to the first instance of a component found.
     pub fn get_single<T: 'static>(&self) -> Result<Single<T>, FetchError> {
        <&T>::fetch(self)
    }

    /// Query for a *mutable* reference to the first instance of a component found.
    pub fn get_single_mut<T: 'static>(&self) -> Result<SingleMut<T>, FetchError> {
        <&mut T>::fetch(self)
    }

    /// ## Example
    /// ```
    /// let query = world.query::<(&bool, &String)>();
    /// ```
    pub fn query<'world_borrow, T: QueryParameters>(&'world_borrow self) -> Result<Query<T>, FetchError> {
        Ok(QueryFetch::<T>::fetch(self)?.take().unwrap())
    }
}

/// A bundle of components. Used to genericize tupled components argument in `World.spawn()`.
pub trait ComponentBundle: 'static + Send + Sync {
    fn new_archetype(&self) -> Archetype;
    fn spawn_in_world(self, world: &mut World, entity_index: EntityId) -> EntityLocation;
}

/// Used in `World.add_component()` and `World.remove_component()`.
fn calculate_bundle_id(types: &[TypeId]) -> u64 {
    let mut s = DefaultHasher::new();
    types.hash(&mut s);
    
    s.finish()
}

macro_rules! component_bundle_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl< $($name: 'static + Send + Sync),*> ComponentBundle for ($($name,)*) {
            fn new_archetype(&self) -> Archetype {
                let mut components = vec![$(ComponentStore::new::<$name>()), *];
                components.sort_unstable_by(|a, b| a.type_id.cmp(&b.type_id));
                Archetype { components, entities: Vec::new() }
            }

            fn spawn_in_world(self, world: &mut World, entity_index: EntityId) -> EntityLocation {
                let mut types = [$(($index, TypeId::of::<$name>())), *];
                types.sort_unstable_by(|a, b| a.1.cmp(&b.1));
                debug_assert!(
                    types.windows(2).all(|x| x[0].1 != x[1].1),
                    "`ComponentBundle`s cannot have duplicate types"
                );

                // Is there a better way to map the original ordering to the sorted ordering?
                let mut order = [0; $count];
                for i in 0..order.len() {
                    order[types[i].0] = i;
                }
                let types = [$(types[$index].1), *];

                let bundle_id = calculate_bundle_id(&types);

                // Find the appropriate archetype
                // If it doesn't exist create a new archetype.
                let archetype_index = if let Some(archetype) = world.bundle_id_to_archetype.get(&bundle_id) {
                    *archetype
                } else {
                    let archetype = self.new_archetype();
                    let index = world.archetypes.len();

                    world.bundle_id_to_archetype.insert(bundle_id, index);
                    world.archetypes.push(archetype);
                    index
                };

                world.archetypes[archetype_index].entities.push(entity_index);
                $(world.archetypes[archetype_index].push(order[$index], self.$index);)*
                EntityLocation {
                    archetype_index: archetype_index as EntityId,
                    index_in_archetype: (world.archetypes[archetype_index].len() - 1) as EntityId
                }
            }
        }
    }
}

component_bundle_impl! {1, (A, 0)}
component_bundle_impl! {2, (A, 0), (B, 1)}
component_bundle_impl! {3, (A, 0), (B, 1), (C, 2)}
component_bundle_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
component_bundle_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
component_bundle_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
component_bundle_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
component_bundle_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
component_bundle_impl! {9, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8)}
component_bundle_impl! {10, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9)}
component_bundle_impl! {11, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10)}
component_bundle_impl! {12, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11)}

/// A helper to get two mutable borrows from the same slice.
fn index_twice<T>(slice: &mut [T], first: usize, second: usize) -> (&mut T, &mut T) {
    if first < second {
        let (a, b) = slice.split_at_mut(second);
        (&mut a[first], &mut b[0])
    } else {
        let (a, b) = slice.split_at_mut(first);
        (&mut b[0], &mut a[second])
    }
}

/// This `Entity` has been despawned so operations can no longer be performed on it.
#[derive(Debug)]
pub struct NoSuchEntity;
#[derive(Debug)]
pub struct EntityMissingComponent(EntityId, &'static str);

impl std::fmt::Display for NoSuchEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the entity no longer exists, operation cannot be performed")
    }
}
impl std::fmt::Display for EntityMissingComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "entity {:?} does not have a [{}] component", self.0, self.1)
    }
}

impl std::error::Error for NoSuchEntity {}
impl std::error::Error for EntityMissingComponent {}

impl EntityMissingComponent {
    pub fn new<T>(entity_id: EntityId) -> Self {
        Self(entity_id, std::any::type_name::<T>())
    }
}

#[derive(Debug)]
pub enum ComponentError {
    EntityMissingComponent(EntityMissingComponent),
    NoSuchEntity(NoSuchEntity),
}
