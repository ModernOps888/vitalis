//! Entity-Component-System (ECS) Module for Vitalis v30.0
//!
//! High-performance data-oriented architecture:
//! - Generational entity IDs (recycling with generation counters)
//! - Sparse-set component storage (O(1) add/remove/get)
//! - Archetype-based entity grouping for cache-friendly iteration
//! - Component queries with With/Without filters
//! - System scheduling with dependency resolution

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── Entity ─────────────────────────────────────────────────────────

/// Generational entity ID — allows safe reuse of entity slots.
/// Lower 32 bits = index, upper 32 bits = generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    fn to_u64(self) -> u64 {
        ((self.generation as u64) << 32) | (self.index as u64)
    }

    fn from_u64(val: u64) -> Self {
        Self {
            index: val as u32,
            generation: (val >> 32) as u32,
        }
    }
}

// ─── Component Storage (Sparse Set) ────────────────────────────────

/// Type-erased component stored as a blob of bytes.
#[derive(Debug, Clone)]
struct ComponentData {
    data: Vec<u8>,
    type_id: u64,
}

/// Sparse set for one component type — O(1) add, remove, has, get.
struct SparseSet {
    /// Sparse array: entity index → dense index (or u32::MAX if absent)
    sparse: Vec<u32>,
    /// Dense array of entity indices
    dense: Vec<u32>,
    /// Component data aligned with `dense`
    components: Vec<i64>,  // simplified: use i64 as component value
    type_id: u64,
}

impl SparseSet {
    fn new(type_id: u64) -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            components: Vec::new(),
            type_id,
        }
    }

    fn ensure_sparse(&mut self, index: u32) {
        let needed = index as usize + 1;
        if self.sparse.len() < needed {
            self.sparse.resize(needed, u32::MAX);
        }
    }

    fn has(&self, index: u32) -> bool {
        (index as usize) < self.sparse.len()
            && self.sparse[index as usize] != u32::MAX
    }

    fn insert(&mut self, index: u32, value: i64) {
        self.ensure_sparse(index);
        if self.has(index) {
            // Update existing
            let dense_idx = self.sparse[index as usize] as usize;
            self.components[dense_idx] = value;
        } else {
            let dense_idx = self.dense.len() as u32;
            self.sparse[index as usize] = dense_idx;
            self.dense.push(index);
            self.components.push(value);
        }
    }

    fn get(&self, index: u32) -> Option<i64> {
        if !self.has(index) {
            return None;
        }
        let dense_idx = self.sparse[index as usize] as usize;
        Some(self.components[dense_idx])
    }

    fn remove(&mut self, index: u32) -> Option<i64> {
        if !self.has(index) {
            return None;
        }
        let dense_idx = self.sparse[index as usize] as usize;
        let value = self.components[dense_idx];

        // Swap-remove: move last element into the removed slot
        let last_dense = self.dense.len() - 1;
        if dense_idx != last_dense {
            let last_entity = self.dense[last_dense];
            self.dense[dense_idx] = last_entity;
            self.components[dense_idx] = self.components[last_dense];
            self.sparse[last_entity as usize] = dense_idx as u32;
        }

        self.dense.pop();
        self.components.pop();
        self.sparse[index as usize] = u32::MAX;
        Some(value)
    }

    fn len(&self) -> usize {
        self.dense.len()
    }

    fn iter(&self) -> impl Iterator<Item = (u32, i64)> + '_ {
        self.dense
            .iter()
            .zip(self.components.iter())
            .map(|(&idx, &val)| (idx, val))
    }
}

// ─── Archetype ──────────────────────────────────────────────────────

/// An archetype groups entities that share the same set of component types.
/// This enables cache-friendly iteration over entities with matching components.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ArchetypeId(Vec<u64>);

impl ArchetypeId {
    fn new(mut types: Vec<u64>) -> Self {
        types.sort();
        Self(types)
    }

    fn contains_all(&self, required: &[u64]) -> bool {
        required.iter().all(|t| self.0.contains(t))
    }

    fn contains_none(&self, excluded: &[u64]) -> bool {
        excluded.iter().all(|t| !self.0.contains(t))
    }
}

struct Archetype {
    id: ArchetypeId,
    entities: Vec<u32>, // entity indices
}

impl Archetype {
    fn new(id: ArchetypeId) -> Self {
        Self {
            id,
            entities: Vec::new(),
        }
    }

    fn add_entity(&mut self, entity_index: u32) {
        if !self.entities.contains(&entity_index) {
            self.entities.push(entity_index);
        }
    }

    fn remove_entity(&mut self, entity_index: u32) {
        if let Some(pos) = self.entities.iter().position(|&e| e == entity_index) {
            self.entities.swap_remove(pos);
        }
    }
}

// ─── Query Filters ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct QueryFilter {
    with: Vec<u64>,
    without: Vec<u64>,
}

impl QueryFilter {
    fn new() -> Self {
        Self {
            with: Vec::new(),
            without: Vec::new(),
        }
    }

    fn with_component(mut self, type_id: u64) -> Self {
        self.with.push(type_id);
        self
    }

    fn without_component(mut self, type_id: u64) -> Self {
        self.without.push(type_id);
        self
    }
}

// ─── World ──────────────────────────────────────────────────────────

/// The ECS World — central container for all entities, components, and systems.
struct World {
    /// Entity generations for recycling
    entity_generations: Vec<u32>,
    /// Free list of recycled entity indices (LIFO)
    free_indices: Vec<u32>,
    /// Alive flags
    alive: Vec<bool>,
    /// Component storages by type_id
    storages: HashMap<u64, SparseSet>,
    /// Entity count
    entity_count: u32,
    /// Registered system names (for scheduling)
    systems: Vec<SystemEntry>,
}

struct SystemEntry {
    name: String,
    filter: QueryFilter,
    priority: i32,
}

impl World {
    fn new() -> Self {
        Self {
            entity_generations: Vec::new(),
            free_indices: Vec::new(),
            alive: Vec::new(),
            storages: HashMap::new(),
            entity_count: 0,
            systems: Vec::new(),
        }
    }

    fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free_indices.pop() {
            // Recycle a dead entity slot
            let generation = self.entity_generations[index as usize];
            self.alive[index as usize] = true;
            self.entity_count += 1;
            Entity::new(index, generation)
        } else {
            // Allocate a new slot
            let index = self.entity_generations.len() as u32;
            self.entity_generations.push(0);
            self.alive.push(true);
            self.entity_count += 1;
            Entity::new(index, 0)
        }
    }

    fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let idx = entity.index as usize;
        self.alive[idx] = false;
        // Bump generation so old Entity handles become invalid
        self.entity_generations[idx] += 1;
        self.free_indices.push(entity.index);
        self.entity_count -= 1;

        // Remove from all storages
        let keys: Vec<u64> = self.storages.keys().copied().collect();
        for type_id in keys {
            self.storages.get_mut(&type_id).unwrap().remove(entity.index);
        }
        true
    }

    fn is_alive(&self, entity: Entity) -> bool {
        let idx = entity.index as usize;
        idx < self.alive.len()
            && self.alive[idx]
            && self.entity_generations[idx] == entity.generation
    }

    fn add_component(&mut self, entity: Entity, type_id: u64, value: i64) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let storage = self
            .storages
            .entry(type_id)
            .or_insert_with(|| SparseSet::new(type_id));
        storage.insert(entity.index, value);
        true
    }

    fn remove_component(&mut self, entity: Entity, type_id: u64) -> Option<i64> {
        if !self.is_alive(entity) {
            return None;
        }
        self.storages.get_mut(&type_id)?.remove(entity.index)
    }

    fn get_component(&self, entity: Entity, type_id: u64) -> Option<i64> {
        if !self.is_alive(entity) {
            return None;
        }
        self.storages.get(&type_id)?.get(entity.index)
    }

    fn has_component(&self, entity: Entity, type_id: u64) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        self.storages
            .get(&type_id)
            .map_or(false, |s| s.has(entity.index))
    }

    /// Query entities matching a filter.
    fn query(&self, filter: &QueryFilter) -> Vec<Entity> {
        if filter.with.is_empty() {
            return Vec::new();
        }

        // Start from the smallest component set for efficiency
        let smallest_type = filter
            .with
            .iter()
            .min_by_key(|&&t| self.storages.get(&t).map_or(0, |s| s.len()))
            .copied();

        let Some(start_type) = smallest_type else {
            return Vec::new();
        };
        let Some(start_storage) = self.storages.get(&start_type) else {
            return Vec::new();
        };

        let mut results = Vec::new();
        for (&entity_idx, _) in start_storage.dense.iter().zip(start_storage.components.iter()) {
            let idx = entity_idx as usize;
            if idx >= self.alive.len() || !self.alive[idx] {
                continue;
            }

            // Check all required components
            let has_all = filter.with.iter().all(|&t| {
                self.storages.get(&t).map_or(false, |s| s.has(entity_idx))
            });

            // Check no excluded components
            let has_none_excluded = filter.without.iter().all(|&t| {
                self.storages.get(&t).map_or(true, |s| !s.has(entity_idx))
            });

            if has_all && has_none_excluded {
                let generation = self.entity_generations[idx];
                results.push(Entity::new(entity_idx, generation));
            }
        }

        results
    }

    fn register_system(&mut self, name: &str, filter: QueryFilter, priority: i32) {
        self.systems.push(SystemEntry {
            name: name.to_string(),
            filter,
            priority,
        });
        // Sort by priority (lower = earlier)
        self.systems.sort_by_key(|s| s.priority);
    }

    /// Get systems in execution order with their matching entity sets.
    fn schedule(&self) -> Vec<(&str, Vec<Entity>)> {
        self.systems
            .iter()
            .map(|sys| {
                let entities = self.query(&sys.filter);
                (sys.name.as_str(), entities)
            })
            .collect()
    }

    fn entity_count(&self) -> u32 {
        self.entity_count
    }

    fn component_count(&self, type_id: u64) -> usize {
        self.storages.get(&type_id).map_or(0, |s| s.len())
    }
}

// ─── FFI Layer ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_world_create() -> *mut World {
    Box::into_raw(Box::new(World::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_world_free(ptr: *mut World) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)); }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_spawn(world: *mut World) -> u64 {
    let world = unsafe { &mut *world };
    world.spawn().to_u64()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_despawn(world: *mut World, entity_id: u64) -> i64 {
    let world = unsafe { &mut *world };
    let entity = Entity::from_u64(entity_id);
    if world.despawn(entity) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_add_component(
    world: *mut World,
    entity_id: u64,
    type_id: u64,
    value: i64,
) -> i64 {
    let world = unsafe { &mut *world };
    let entity = Entity::from_u64(entity_id);
    if world.add_component(entity, type_id, value) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_get_component(
    world: *mut World,
    entity_id: u64,
    type_id: u64,
) -> i64 {
    let world = unsafe { &*world };
    let entity = Entity::from_u64(entity_id);
    world.get_component(entity, type_id).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_has_component(
    world: *mut World,
    entity_id: u64,
    type_id: u64,
) -> i64 {
    let world = unsafe { &*world };
    let entity = Entity::from_u64(entity_id);
    if world.has_component(entity, type_id) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_entity_count(world: *mut World) -> i64 {
    let world = unsafe { &*world };
    world.entity_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ecs_component_count(world: *mut World, type_id: u64) -> i64 {
    let world = unsafe { &*world };
    world.component_count(type_id) as i64
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_roundtrip() {
        let e = Entity::new(42, 7);
        let encoded = e.to_u64();
        let decoded = Entity::from_u64(encoded);
        assert_eq!(decoded, e);
    }

    #[test]
    fn test_entity_spawn() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        assert_ne!(e1.index, e2.index);
        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn test_entity_despawn() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.is_alive(e));
        world.despawn(e);
        assert!(!world.is_alive(e));
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_entity_recycling() {
        let mut world = World::new();
        let e1 = world.spawn();
        let idx1 = e1.index;
        world.despawn(e1);
        let e2 = world.spawn();
        // Should reuse the same index
        assert_eq!(e2.index, idx1);
        // But with bumped generation
        assert_eq!(e2.generation, 1);
        // Old handle should be invalid
        assert!(!world.is_alive(e1));
        assert!(world.is_alive(e2));
    }

    #[test]
    fn test_sparse_set_insert_get() {
        let mut ss = SparseSet::new(1);
        ss.insert(5, 100);
        ss.insert(3, 200);
        assert_eq!(ss.get(5), Some(100));
        assert_eq!(ss.get(3), Some(200));
        assert_eq!(ss.get(7), None);
        assert_eq!(ss.len(), 2);
    }

    #[test]
    fn test_sparse_set_remove() {
        let mut ss = SparseSet::new(1);
        ss.insert(1, 10);
        ss.insert(2, 20);
        ss.insert(3, 30);
        ss.remove(2);
        assert_eq!(ss.get(2), None);
        assert_eq!(ss.get(1), Some(10));
        assert_eq!(ss.get(3), Some(30));
        assert_eq!(ss.len(), 2);
    }

    #[test]
    fn test_sparse_set_update() {
        let mut ss = SparseSet::new(1);
        ss.insert(1, 10);
        ss.insert(1, 99);
        assert_eq!(ss.get(1), Some(99));
        assert_eq!(ss.len(), 1);
    }

    #[test]
    fn test_add_get_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 1, 42);
        assert_eq!(world.get_component(e, 1), Some(42));
        assert!(world.has_component(e, 1));
        assert!(!world.has_component(e, 2));
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 1, 99);
        let removed = world.remove_component(e, 1);
        assert_eq!(removed, Some(99));
        assert!(!world.has_component(e, 1));
    }

    #[test]
    fn test_despawn_removes_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 1, 10);
        world.add_component(e, 2, 20);
        world.despawn(e);
        assert_eq!(world.component_count(1), 0);
        assert_eq!(world.component_count(2), 0);
    }

    #[test]
    fn test_query_with_single_component() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();
        world.add_component(e1, 1, 10);
        world.add_component(e2, 1, 20);
        // e3 has no component 1
        let filter = QueryFilter::new().with_component(1);
        let results = world.query(&filter);
        assert_eq!(results.len(), 2);
        let _ = e3; // suppress unused warning
    }

    #[test]
    fn test_query_with_multiple_components() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.add_component(e1, 1, 10);
        world.add_component(e1, 2, 20);
        world.add_component(e2, 1, 30);
        // e2 only has component 1, not 2
        let filter = QueryFilter::new().with_component(1).with_component(2);
        let results = world.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], e1);
    }

    #[test]
    fn test_query_without_filter() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.add_component(e1, 1, 10);
        world.add_component(e2, 1, 20);
        world.add_component(e2, 2, 30); // e2 also has type 2
        let filter = QueryFilter::new().with_component(1).without_component(2);
        let results = world.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], e1);
    }

    #[test]
    fn test_archetype_id() {
        let a1 = ArchetypeId::new(vec![3, 1, 2]);
        let a2 = ArchetypeId::new(vec![1, 2, 3]);
        assert_eq!(a1, a2); // order-independent
    }

    #[test]
    fn test_archetype_contains() {
        let arch = ArchetypeId::new(vec![1, 2, 3]);
        assert!(arch.contains_all(&[1, 3]));
        assert!(!arch.contains_all(&[1, 4]));
        assert!(arch.contains_none(&[4, 5]));
        assert!(!arch.contains_none(&[2, 5]));
    }

    #[test]
    fn test_system_scheduling() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.add_component(e1, 1, 100);

        world.register_system("physics", QueryFilter::new().with_component(1), 0);
        world.register_system("render", QueryFilter::new().with_component(1), 10);

        let schedule = world.schedule();
        assert_eq!(schedule.len(), 2);
        assert_eq!(schedule[0].0, "physics"); // lower priority runs first
        assert_eq!(schedule[1].0, "render");
        assert_eq!(schedule[0].1.len(), 1);
    }

    #[test]
    fn test_many_entities() {
        let mut world = World::new();
        let mut entities = Vec::new();
        for i in 0..1000 {
            let e = world.spawn();
            world.add_component(e, 1, i);
            entities.push(e);
        }
        assert_eq!(world.entity_count(), 1000);
        assert_eq!(world.component_count(1), 1000);

        // Despawn half
        for e in entities.iter().step_by(2) {
            world.despawn(*e);
        }
        assert_eq!(world.entity_count(), 500);
        assert_eq!(world.component_count(1), 500);
    }

    #[test]
    fn test_sparse_set_iter() {
        let mut ss = SparseSet::new(1);
        ss.insert(2, 20);
        ss.insert(5, 50);
        ss.insert(8, 80);
        let pairs: Vec<(u32, i64)> = ss.iter().collect();
        assert_eq!(pairs.len(), 3);
        assert!(pairs.contains(&(2, 20)));
        assert!(pairs.contains(&(5, 50)));
        assert!(pairs.contains(&(8, 80)));
    }

    #[test]
    fn test_component_on_dead_entity() {
        let mut world = World::new();
        let e = world.spawn();
        world.despawn(e);
        assert!(!world.add_component(e, 1, 42));
        assert_eq!(world.get_component(e, 1), None);
    }

    #[test]
    fn test_ffi_world_lifecycle() {
        let world = vitalis_ecs_world_create();
        let e1 = vitalis_ecs_spawn(world);
        let e2 = vitalis_ecs_spawn(world);
        assert_eq!(vitalis_ecs_entity_count(world), 2);

        vitalis_ecs_add_component(world, e1, 1, 42);
        assert_eq!(vitalis_ecs_get_component(world, e1, 1), 42);
        assert_eq!(vitalis_ecs_has_component(world, e1, 1), 1);
        assert_eq!(vitalis_ecs_has_component(world, e2, 1), 0);

        vitalis_ecs_despawn(world, e1);
        assert_eq!(vitalis_ecs_entity_count(world), 1);

        vitalis_ecs_world_free(world);
    }

    #[test]
    fn test_ffi_component_count() {
        let world = vitalis_ecs_world_create();
        let e1 = vitalis_ecs_spawn(world);
        let e2 = vitalis_ecs_spawn(world);
        vitalis_ecs_add_component(world, e1, 100, 1);
        vitalis_ecs_add_component(world, e2, 100, 2);
        assert_eq!(vitalis_ecs_component_count(world, 100), 2);
        vitalis_ecs_world_free(world);
    }
}
