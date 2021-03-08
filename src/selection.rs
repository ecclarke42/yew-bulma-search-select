use std::{
    collections::BTreeSet,
    fmt::Display,
    sync::{Arc, RwLock},
};

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

    /// Create `SelectOptions` with this mode
    pub fn with_options<T, I: IntoIterator<Item = T>>(self, options: I) -> crate::SelectOptions<T> {
        Arc::new(SelectState {
            options: options.into_iter().map(Arc::new).collect::<Vec<_>>(), // Arc::new(RwLock::new(options)),
            selected_indices: RwLock::new(self),
            filtered_indices: RwLock::new(Filtered::All),
        })
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
    fn select(&mut self, index: usize) -> bool {
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
    fn deselect(&mut self, index: usize) -> bool {
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
    pub fn clear(&mut self) -> bool {
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

#[derive(Debug)]
pub enum Filtered {
    None,
    Some(BTreeSet<usize>),
    All,
}

/// Primarily used inside an Arc
pub struct SelectState<T> {
    pub(crate) options: Vec<Arc<T>>,
    pub(crate) selected_indices: RwLock<Selection>,
    pub(crate) filtered_indices: RwLock<Filtered>,
}

impl<T> SelectState<T>
where
    T: Display,
{
    pub fn is_multiple(&self) -> bool {
        if let Ok(inner) = self.selected_indices.read() {
            inner.is_multiple()
        } else {
            // TODO: handle lock error?
            false
        }
    }

    pub fn is_nullable(&self) -> bool {
        if let Ok(inner) = self.selected_indices.read() {
            inner.is_nullable()
        } else {
            // TODO: handle lock error?
            false
        }
    }

    pub fn first_selected(&self) -> Option<(usize, Arc<T>)> {
        if let Ok(selected) = self.selected_indices.read() {
            match *selected {
                Selection::MaybeOne(None) => {}
                Selection::AlwaysOne(index) | Selection::MaybeOne(Some(index)) => {
                    if let Some(item) = self.options.get(index) {
                        return Some((index, item.clone()));
                    }
                }
                Selection::Multiple(ref set) => {
                    if let Some(&index) = set.iter().next() {
                        if let Some(item) = self.options.get(index) {
                            return Some((index, item.clone()));
                        }
                    }
                }
            }
        }

        None
    }

    pub fn selected_items(&self) -> Vec<(usize, Arc<T>)> {
        if let Ok(selected) = self.selected_indices.read() {
            match *selected {
                Selection::MaybeOne(None) => Vec::new(),
                Selection::AlwaysOne(index) | Selection::MaybeOne(Some(index)) => {
                    if let Some(item) = self.options.get(index) {
                        vec![(index, item.clone())]
                    } else {
                        Vec::new()
                    }
                }
                Selection::Multiple(ref set) => {
                    // let mut indices = set.iter().cloned().collect::<Vec<_>>();
                    // indices.sort_unstable();

                    let mut selected_items = Vec::with_capacity(set.len());
                    for &index in set {
                        if let Some(item) = self.options.get(index) {
                            selected_items.push((index, item.clone()))
                        }
                    }
                    selected_items
                }
            }
        } else {
            Vec::new()
        }
    }

    pub fn first_filtered(&self) -> Option<(usize, Arc<T>)> {
        if let Ok(filtered) = self.filtered_indices.read() {
            match *filtered {
                Filtered::All => {
                    if let Some(item) = self.options.first() {
                        return Some((0, item.clone()));
                    }
                }
                Filtered::Some(ref set) => {
                    if let Some(&index) = set.iter().next() {
                        if let Some(item) = self.options.get(index) {
                            return Some((index, item.clone()));
                        }
                    }
                }
                Filtered::None => {}
            }
        }

        None
    }

    // Get an option item an it's global index using it's relative position in the filter list
    pub fn get_filtered(&self, position: usize) -> Option<(usize, Arc<T>)> {
        if let Ok(filtered) = self.filtered_indices.read() {
            match *filtered {
                Filtered::All => {
                    // If no filtering, position is equivalent to index
                    if let Some(item) = self.options.get(position) {
                        return Some((position, item.clone()));
                    }
                }
                Filtered::Some(ref set) => {
                    // If filtered, we need to find the global index of the item at this position
                    if let Some(&index) = set.iter().nth(position) {
                        if let Some(item) = self.options.get(index) {
                            return Some((index, item.clone()));
                        }
                    }
                }
                Filtered::None => {} // No elements means nothing at this position
            }
        }

        None
    }

    pub fn filtered_items(&self) -> Vec<(usize, bool, Arc<T>)> {
        if let (Ok(filtered), Ok(selected)) =
            (self.filtered_indices.read(), self.selected_indices.read())
        {
            match *filtered {
                Filtered::All => self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, item)| (i, selected.includes(&i), item.clone()))
                    .collect::<Vec<_>>(),
                Filtered::Some(ref set) => {
                    // let mut indices = set.iter().cloned().collect::<Vec<_>>();
                    // indices.sort_unstable();

                    let mut filtered_items = Vec::with_capacity(set.len());
                    for &index in set {
                        if let Some(item) = self.options.get(index) {
                            filtered_items.push((index, selected.includes(&index), item.clone()))
                        }
                    }
                    filtered_items
                }

                Filtered::None => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    /// Select an index from the options.
    /// Returns true if the selection has changed.
    pub fn select(&self, index: usize) -> bool {
        if index >= self.options.len() {
            return false;
        }

        if let Ok(mut inner) = self.selected_indices.write() {
            inner.select(index)
        } else {
            false
        }
    }

    /// Deselect an index from the options.
    /// Returns true if the selection has changed.
    pub fn deselect(&self, index: usize) -> bool {
        if index >= self.options.len() {
            return false;
        }

        if let Ok(mut inner) = self.selected_indices.write() {
            inner.deselect(index)
        } else {
            false
        }
    }

    /// Clear the selected items.
    /// Returns true if the selection has changed.
    pub fn clear(&self) -> bool {
        if let Ok(mut inner) = self.selected_indices.write() {
            inner.clear()
        } else {
            false
        }
    }

    pub async fn filter<F: Fn(&T) -> bool>(&self, filter_fn: F) {
        if let Ok(mut inner) = self.filtered_indices.write() {
            let indices = self
                .options
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if (filter_fn)(item.as_ref()) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect::<BTreeSet<usize>>();

            *inner = if indices.is_empty() {
                Filtered::None
            } else {
                Filtered::Some(indices)
            }
        }
    }

    pub async fn unfilter(&self) {
        if let Ok(mut inner) = self.filtered_indices.write() {
            *inner = Filtered::All;
        }
    }
}
