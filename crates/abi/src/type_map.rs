//! # Type Map

use {
    core::{
        any::TypeId,
        hash::{BuildHasherDefault, Hasher},
    },
    std::collections::{
        HashMap,
        hash_map::{Drain, Entry},
    },
};



/// A map that can store homogenous values for any number of types.
pub struct TypeMap<V> {
    map: HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>,
}

pub type TypeMapEntry<'a, V> = Entry<'a, TypeId, V>;

impl<V> Default for TypeMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> TypeMap<V> {
    /// Create an empty type map.
    pub const fn new() -> Self {
        Self {
            map: HashMap::with_hasher(BuildHasherDefault::new()),
        }
    }

    /// Clear all values from this map.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Clear the map, returning all [`TypeId`]-`V` pairs as an iterator. Keeps
    /// the allocated memory for future reuse.
    pub fn drain(&mut self) -> Drain<'_, TypeId, V> {
        self.map.drain()
    }
}

impl<V> TypeMap<V> {
    /// Whether the type `K` has a value stored in this map.
    pub fn has<K: ?Sized + 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<K>())
    }

    /// Insert the value `V` associated with `K` into this map.
    pub fn insert<K: ?Sized + 'static>(&mut self, value: V) {
        let _ = self.map.insert(TypeId::of::<K>(), value);
    }

    /// Get the associated value `V` for `K`, if any.
    pub fn get<K: ?Sized + 'static>(&self) -> Option<&V> {
        self.map.get(&TypeId::of::<K>())
    }

    /// Get a mutable reference to the associated value `V` for `K`, if any.
    pub fn get_mut<K: ?Sized + 'static>(&mut self) -> Option<&mut V> {
        self.map.get_mut(&TypeId::of::<K>())
    }

    /// Gets the given key’s corresponding [entry](TypeMapEntry) in this map for
    /// in-place manipulation.
    pub fn entry<K: ?Sized + 'static>(&mut self) -> TypeMapEntry<'_, V> {
        self.map.entry(TypeId::of::<K>())
    }
}

impl<V> TypeMap<V> {
    /// Whether the type identified by the given [`TypeId`] has a value stored
    /// in this map.
    pub fn has_id(&self, id: TypeId) -> bool {
        self.map.contains_key(&id)
    }

    /// Insert the value `V` associated with the given [`TypeId`] into this map.
    pub fn insert_for_id(&mut self, id: TypeId, value: V) {
        let _ = self.map.insert(id, value);
    }

    /// Get the associated value `V` for the given [`TypeId`], if any.
    pub fn get_for_id(&self, id: TypeId) -> Option<&V> {
        self.map.get(&id)
    }

    /// Get a mutable reference to the associated value `V` for the given
    /// [`TypeId`], if any.
    pub fn get_mut_for_id(&mut self, id: TypeId) -> Option<&mut V> {
        self.map.get_mut(&id)
    }

    /// Gets the given [`TypeId`]’s corresponding [entry](TypeMapEntry) in this
    /// map for in-place manipulation.
    pub fn entry_for_id(&mut self, id: TypeId) -> TypeMapEntry<'_, V> {
        self.map.entry(id)
    }
}


/// A hashless hasher designed specifically with [`TypeId`]s in mind.
#[derive(Default)]
pub struct TypeIdHasher {
    hash: u64,
}

impl Hasher for TypeIdHasher {
    fn write_u64(&mut self, n: u64) {
        // Only a single value can be hashed, so the old hash should be zero.
        debug_assert_eq!(self.hash, 0);
        self.hash = n;
    }

    fn write_u128(&mut self, n: u128) {
        debug_assert_eq!(self.hash, 0);
        self.hash = n as u64;
    }

    fn write(&mut self, _bytes: &[u8]) {
        panic!("Type ID is the wrong type!")
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    struct A;
    struct B;
    struct C;

    #[test]
    fn type_map_works() {
        let mut map = TypeMap::<usize>::new();

        map.insert::<A>(1);
        map.insert::<B>(2);
        map.insert::<C>(3);

        assert!(map.get::<A>().is_some_and(|i| *i == 1));
        assert!(map.get::<B>().is_some_and(|i| *i == 2));
        assert!(map.get::<C>().is_some_and(|i| *i == 3));

        assert!(map.drain().count() == 3);
    }
}
