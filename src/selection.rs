use std::collections::BTreeSet;

// TODO: evaluate performance of using btreemap's instead of sets (it's nice to have the sortedness, but performance?)
// insertion should (almost always) be a greater value?

#[derive(Clone)]
pub enum Selection {
    AlwaysOne(usize),
    MaybeOne(Option<usize>),
    Multiple(BTreeSet<usize>),
}

impl Selection {
    /// Create a new `Selection::AlwaysOne` with the given selection
    pub fn one(index: usize) -> Self {
        Selection::AlwaysOne(index)
    }

    /// Create a new `Selection::MaybeOne` with the given selection
    pub fn some(index: usize) -> Self {
        Selection::MaybeOne(Some(index))
    }

    /// Create a new `Selection::MaybeOne` with no selection
    pub fn none() -> Self {
        Selection::MaybeOne(None)
    }

    /// Create a new `Selection::Multiple` with no selection
    pub fn empty() -> Self {
        Selection::Multiple(BTreeSet::new())
    }

    /// Create a new `Selection::Multiple` with some indices selected
    pub fn multiple<T>(indices: T) -> Self
    where
        T: IntoIterator<Item = usize>,
    {
        Selection::Multiple(indices.into_iter().collect::<BTreeSet<usize>>())
    }

    pub fn len(&self) -> usize {
        match self {
            Selection::MaybeOne(None) => 0,
            Selection::AlwaysOne(_) | Selection::MaybeOne(Some(_)) => 1,
            Selection::Multiple(ref set) => set.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Selection::MaybeOne(None) => true,
            Selection::AlwaysOne(_) | Selection::MaybeOne(Some(_)) => false,
            Selection::Multiple(ref set) => set.is_empty(),
        }
    }

    pub fn as_set(&self) -> BTreeSet<usize> {
        match self {
            Selection::MaybeOne(None) => BTreeSet::new(),
            Selection::AlwaysOne(index) | Selection::MaybeOne(Some(index)) => {
                let mut set = BTreeSet::new();
                set.insert(*index);
                set
            }
            Selection::Multiple(ref set) => set.clone(),
        }
    }

    pub(crate) fn includes(&self, i: &usize) -> bool {
        match self {
            Selection::MaybeOne(None) => false,
            Selection::AlwaysOne(index) | Selection::MaybeOne(Some(index)) => *index == *i,
            Selection::Multiple(ref set) => set.contains(i),
        }
    }

    pub fn is_multiple(&self) -> bool {
        match self {
            Selection::AlwaysOne(_) => false,
            Selection::MaybeOne(_) => false,
            Selection::Multiple(_) => true,
        }
    }

    pub fn is_nullable(&self) -> bool {
        match self {
            Selection::AlwaysOne(_) => false,
            Selection::MaybeOne(_) => true,
            Selection::Multiple(_) => true,
        }
    }

    /// Select an index from the options.
    /// Returns true if the selection has changed.
    pub(crate) fn select(&mut self, index: usize) -> bool {
        match self {
            Selection::AlwaysOne(ref mut idx) => {
                if index != *idx {
                    *idx = index;
                    true
                } else {
                    false
                }
            }
            Selection::MaybeOne(ref mut maybe_idx) => {
                if let Some(old_index) = maybe_idx.replace(index) {
                    old_index != index
                } else {
                    // Was `None`, now `Some(_)`
                    true
                }
            }
            Selection::Multiple(ref mut set) => set.insert(index),
        }
    }

    /// Deselect an index from the options.
    /// Returns true if the selection has changed.
    pub(crate) fn deselect(&mut self, index: usize) -> bool {
        match self {
            Selection::AlwaysOne(_) => false, // Cannot deselect AlwaysOne
            Selection::MaybeOne(None) => false,
            Selection::MaybeOne(Some(i)) => {
                if index == *i {
                    *self = Selection::MaybeOne(None);
                    true
                } else {
                    // If mismatched index, do nothing
                    false
                }
            }
            Selection::Multiple(ref mut set) => set.remove(&index),
        }
    }

    /// Clear the selected items.
    /// Returns true if the selection has changed.
    pub(crate) fn clear(&mut self) -> bool {
        match self {
            Selection::AlwaysOne(_) => false, // Cannot deselect AlwaysOne
            Selection::MaybeOne(None) => false,
            Selection::MaybeOne(Some(_)) => {
                *self = Selection::MaybeOne(None);
                true
            }
            Selection::Multiple(ref mut set) => {
                if set.is_empty() {
                    false
                } else {
                    set.clear();
                    true
                }
            }
        }
    }
}
