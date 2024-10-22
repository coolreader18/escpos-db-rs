use std::borrow::Borrow;
use std::ops::Deref;
use std::{fmt, mem};

/// A mapping from a `u8` to `T`.
///
/// Printer capabilities represented with this type will generally involve
/// sending a key in this map as part of a command to the printer. For example,
/// for [character fonts], sending `[0x1b, b'M']` followed by `0x03` to the
/// printer selects "font D", and correspondingly, `profile.fonts.get(3)` will
/// return information about the printer's "font D", if it exists.
///
/// [character fonts]: crate::Profile::fonts
#[repr(transparent)]
pub struct IntMap<T> {
    // sorted
    entries: [(u8, T)],
}

fn key_f<T>((k, _): &(u8, T)) -> u8 {
    *k
}

impl<T> Default for &IntMap<T> {
    fn default() -> Self {
        IntMap::empty()
    }
}

impl<T> IntMap<T> {
    #[inline]
    const fn from_entries_unchecked(entries: &[(u8, T)]) -> &Self {
        unsafe { &*(entries as *const _ as *const Self) }
    }

    /// An empty `IntMap`.
    #[inline]
    pub const fn empty<'a>() -> &'a Self {
        Self::from_entries_unchecked(&[])
    }

    /// Create an `IntMap` from the given map entries.
    ///
    /// # Panics
    ///
    /// This function will panic if `entries` is not ordered by the first field of the tuple, or has duplicates.
    pub const fn from_entries(entries: &[(u8, T)]) -> &Self {
        if !entries.is_empty() {
            let mut prev = entries[0].0;
            let mut i = 1;
            while i < entries.len() {
                let cur = entries[i].0;
                if cur <= prev {
                    panic!("invalid entries array");
                }
                prev = cur;
                i += 1;
            }
        }
        Self::from_entries_unchecked(entries)
    }

    /// Lookup a value by the given key.
    pub fn get(&self, k: u8) -> Option<&T> {
        let i = self.entries.binary_search_by_key(&k, key_f).ok()?;
        Some(&self.entries[i].1)
    }

    /// Returns an iterator over the entries of this map.
    pub fn iter(&self) -> IntMapIter<'_, T> {
        self.into_iter()
    }
}

impl<T> AsRef<IntMap<T>> for IntMap<T> {
    fn as_ref(&self) -> &IntMap<T> {
        self
    }
}

impl<T: fmt::Debug> fmt::Debug for IntMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self).finish()
    }
}

impl<'a, T> IntoIterator for &'a IntMap<T> {
    type Item = (u8, &'a T);
    type IntoIter = IntMapIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        IntMapIter {
            inner: self.entries.iter(),
        }
    }
}

/// An iterator over an [`IntMap`].
pub struct IntMapIter<'a, T> {
    inner: std::slice::Iter<'a, (u8, T)>,
}

impl<'a, T> Iterator for IntMapIter<'a, T> {
    type Item = (u8, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (*k, v))
    }
}

impl<T> ToOwned for IntMap<T> {
    type Owned = OwnedIntMap<T>;
    fn to_owned(&self) -> Self::Owned {
        todo!()
    }
}

/// An owned version of [`IntMap`].
pub struct OwnedIntMap<T> {
    entries: Vec<(u8, T)>,
}

impl<T> Deref for OwnedIntMap<T> {
    type Target = IntMap<T>;
    fn deref(&self) -> &Self::Target {
        IntMap::from_entries_unchecked(&self.entries)
    }
}

impl<T> Borrow<IntMap<T>> for OwnedIntMap<T> {
    fn borrow(&self) -> &IntMap<T> {
        self
    }
}

impl<T> OwnedIntMap<T> {
    /// Insert a new entry into the map, returning the previous value at `key` if it existed.
    pub fn insert(&mut self, key: u8, val: T) -> Option<T> {
        match self.entries.binary_search_by_key(&key, key_f) {
            Ok(i) => Some(mem::replace(&mut self.entries[i].1, val)),
            Err(i) => {
                self.entries.insert(i, (key, val));
                None
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for OwnedIntMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T> FromIterator<(u8, T)> for OwnedIntMap<T> {
    fn from_iter<I: IntoIterator<Item = (u8, T)>>(iter: I) -> Self {
        let mut entries = iter.into_iter().collect::<Vec<_>>();
        entries.sort_by_key(key_f);
        Self { entries }
    }
}

impl<T> Extend<(u8, T)> for OwnedIntMap<T> {
    fn extend<I: IntoIterator<Item = (u8, T)>>(&mut self, iter: I) {
        struct DropGuard<'a, T> {
            entries: &'a mut Vec<(u8, T)>,
            reset_len: usize,
        }
        impl<T> Drop for DropGuard<'_, T> {
            fn drop(&mut self) {
                self.entries.truncate(self.reset_len);
            }
        }
        let guard = DropGuard {
            reset_len: self.entries.len(),
            entries: &mut self.entries,
        };
        guard.entries.extend(iter);
        mem::forget(guard);
        self.entries.sort_by_key(key_f);
    }
}
