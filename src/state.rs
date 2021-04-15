use std::{
    collections::BTreeSet,
    sync::{Arc, RwLock},
};

use crate::{SelectFilter, Selection};

#[derive(Debug)]
pub enum Filtered {
    None,
    Some(BTreeSet<usize>),
    All,
}

/// Internal state is wrapped in an Arc, so cloning this is not very expensive
pub struct SelectState<T> {
    pub(crate) options: Arc<[T]>,
    pub(crate) selected_indices: Arc<RwLock<Selection>>,
    pub(crate) filtered_indices: Arc<RwLock<Filtered>>,

    filter_fn: SelectFilter<T>,
    filter_input: Arc<RwLock<Option<String>>>,
}

impl<T> Clone for SelectState<T> {
    fn clone(&self) -> Self {
        Self {
            options: self.options.clone(),
            selected_indices: self.selected_indices.clone(),
            filtered_indices: self.filtered_indices.clone(),
            filter_fn: self.filter_fn.clone(),
            filter_input: self.filter_input.clone(),
        }
    }
}

impl<T> PartialEq for SelectState<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.options, &other.options)
            && Arc::ptr_eq(&self.selected_indices, &other.selected_indices)
            && Arc::ptr_eq(&self.filtered_indices, &other.filtered_indices)
            && self.filter_fn == other.filter_fn
            && Arc::ptr_eq(&self.filter_input, &other.filter_input)
    }
}

impl<T> SelectState<T> {
    // TODO: make filter optional?
    pub fn new<I: Into<Arc<[T]>>, F: Into<SelectFilter<T>>>(
        options: I,
        selection: Selection,
        filter_fn: F,
    ) -> Self {
        Self {
            options: options.into(),
            selected_indices: Arc::new(RwLock::new(selection)),
            filtered_indices: Arc::new(RwLock::new(Filtered::All)),

            filter_fn: filter_fn.into(),
            filter_input: Arc::new(RwLock::new(None)),
        }
    }

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

    /// Replace the option set. You should probably use `replace_options_reselecting`
    pub async fn replace_options<I: Into<Arc<[T]>>>(&mut self, options: I) {
        if let Ok(mut inner) = self.selected_indices.write() {
            match *inner {
                Selection::MaybeOne(_) => *inner = Selection::none(),
                Selection::AlwaysOne(_) => *inner = Selection::one(0),
                Selection::Multiple(_) => *inner = Selection::empty(),
            }
        }
        self.refilter().await;
        self.options = options.into();
    }

    /// Replace the existing options and attempt to reeselect the existing selections
    /// (if `Selection::AlwaysOne`, it will default to index 0 if not found)
    pub async fn replace_options_reselecting<I: Into<Arc<[T]>>, F: Fn(&T, &T) -> bool>(
        &mut self,
        options: I,
        selection_eq: F,
    ) {
        let new_options: Arc<[T]> = options.into();
        if let Ok(mut inner) = self.selected_indices.write() {
            match *inner {
                Selection::MaybeOne(None) => {} // Do nothing
                Selection::MaybeOne(Some(index)) => {
                    *inner = Selection::MaybeOne(
                        self.options
                            .get(index)
                            .map(|item| new_options.iter().position(|t| (selection_eq)(item, t)))
                            .flatten(),
                    )
                }
                Selection::AlwaysOne(index) => {
                    *inner = Selection::one(
                        self.options
                            .get(index)
                            .map(|item| new_options.iter().position(|t| (selection_eq)(item, t)))
                            .flatten()
                            .unwrap_or_default(),
                    )
                }
                Selection::Multiple(ref indices) => {
                    *inner = Selection::Multiple(
                        indices
                            .iter()
                            .filter_map(|&i| {
                                self.options
                                    .get(i)
                                    .map(|item| {
                                        new_options.iter().position(|t| (selection_eq)(item, t))
                                    })
                                    .flatten()
                            })
                            .collect(),
                    )
                }
            }
        }
        self.refilter().await;
        self.options = new_options;
    }

    async fn filter_inner(&self, input: &str) {
        if let Ok(mut filtered_indices) = self.filtered_indices.write() {
            let indices = self
                .options
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if self.filter_fn.call(item, input) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect::<BTreeSet<usize>>();

            *filtered_indices = if indices.is_empty() {
                Filtered::None
            } else {
                Filtered::Some(indices)
            }
        }
    }

    async fn refilter(&self) {
        if let Ok(input) = self.filter_input.read() {
            if let Some(ref input) = *input {
                self.filter_inner(input).await;
            } else {
                self.unfilter().await;
            }
        }
        // TODO: handle errors
    }

    pub async fn filter(&self, input: &str) {
        if input.is_empty() {
            if let Ok(mut filter_input) = self.filter_input.write() {
                *filter_input = Some(input.to_string());
            } else {
                // TODO: handle poison
            }
            self.filter_inner(input).await;
        } else {
            if let Ok(mut filter_input) = self.filter_input.write() {
                *filter_input = None;
            } else {
                // TODO: handle poison
            }
            self.unfilter().await
        }
    }

    pub async fn unfilter(&self) {
        if let Ok(mut inner) = self.filtered_indices.write() {
            *inner = Filtered::All;
        }
    }

    // Expose the internal api of the options
    pub fn get(&self, index: usize) -> Option<&T> {
        self.options.get(index)
    }
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.options.iter()
    }

    pub fn first_selected(&self) -> Option<(usize, &T)> {
        if let Ok(selected) = self.selected_indices.read() {
            match *selected {
                Selection::MaybeOne(None) => {}
                Selection::AlwaysOne(index) | Selection::MaybeOne(Some(index)) => {
                    if let Some(item) = self.options.get(index) {
                        return Some((index, item));
                    }
                }
                Selection::Multiple(ref set) => {
                    if let Some(&index) = set.iter().next() {
                        if let Some(item) = self.options.get(index) {
                            return Some((index, item));
                        }
                    }
                }
            }
        }

        None
    }

    pub fn selected_items(&self) -> Vec<(usize, &T)> {
        if let Ok(selected) = self.selected_indices.read() {
            match *selected {
                Selection::MaybeOne(None) => Vec::new(),
                Selection::AlwaysOne(index) | Selection::MaybeOne(Some(index)) => {
                    if let Some(item) = self.options.get(index) {
                        vec![(index, item)]
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
                            selected_items.push((index, item))
                        }
                    }
                    selected_items
                }
            }
        } else {
            Vec::new()
        }
    }

    pub fn first_filtered(&self) -> Option<(usize, &T)> {
        if let Ok(filtered) = self.filtered_indices.read() {
            match *filtered {
                Filtered::All => {
                    if let Some(item) = self.options.first() {
                        return Some((0, item));
                    }
                }
                Filtered::Some(ref set) => {
                    if let Some(&index) = set.iter().next() {
                        if let Some(item) = self.options.get(index) {
                            return Some((index, item));
                        }
                    }
                }
                Filtered::None => {}
            }
        }

        None
    }

    // Get an option item an it's global index using it's relative position in the filter list
    pub fn get_filtered(&self, position: usize) -> Option<(usize, &T)> {
        if let Ok(filtered) = self.filtered_indices.read() {
            match *filtered {
                Filtered::All => {
                    // If no filtering, position is equivalent to index
                    if let Some(item) = self.options.get(position) {
                        return Some((position, item));
                    }
                }
                Filtered::Some(ref set) => {
                    // If filtered, we need to find the global index of the item at this position
                    if let Some(&index) = set.iter().nth(position) {
                        if let Some(item) = self.options.get(index) {
                            return Some((index, item));
                        }
                    }
                }
                Filtered::None => {} // No elements means nothing at this position
            }
        }

        None
    }

    pub fn filtered_items(&self) -> Vec<(usize, bool, &T)> {
        if let (Ok(filtered), Ok(selected)) =
            (self.filtered_indices.read(), self.selected_indices.read())
        {
            match *filtered {
                Filtered::All => self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, item)| (i, selected.includes(&i), item))
                    .collect::<Vec<_>>(),
                Filtered::Some(ref set) => {
                    // let mut indices = set.iter().cloned().collect::<Vec<_>>();
                    // indices.sort_unstable();

                    let mut filtered_items = Vec::with_capacity(set.len());
                    for &index in set {
                        if let Some(item) = self.options.get(index) {
                            filtered_items.push((index, selected.includes(&index), item))
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
}
