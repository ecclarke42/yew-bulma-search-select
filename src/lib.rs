use std::{fmt::Display, sync::Arc};
use yew::prelude::*;
use yewtil::future::LinkFuture;

mod selection;
pub use selection::Selection;

pub type SelectOptions<T> = Arc<selection::SelectState<T>>;
pub type SelectFilter<T> = Arc<dyn Fn(&T, &str) -> bool>;

pub fn filter<T, F: Fn(&T, &str) -> bool + 'static>(f: F) -> SelectFilter<T> {
    Arc::new(f) as SelectFilter<T>
}

/// Bulma-based selection box
/// TODO: document
pub struct Select<T>
where
    T: Clone + PartialEq + Display + 'static,
{
    link: ComponentLink<Self>,
    props: SelectProps<T>,

    focused: bool,
    selection_index: usize,
    search_text: String,
}

#[derive(Properties, Clone)]
pub struct SelectProps<T>
where
    T: Clone + PartialEq + 'static,
{
    /// Omit selected items from the dropdown list (if false, selected will be
    /// highlighted).
    #[prop_or_default]
    pub omit_selected: bool,

    pub options: SelectOptions<T>,
    pub filter: Arc<dyn Fn(&T, &str) -> bool>,

    #[prop_or_default]
    pub onselected: Option<Callback<usize>>,
    #[prop_or_default]
    pub onremoved: Option<Callback<usize>>,

    #[prop_or_else(|| String::from("Type to search"))]
    pub placeholder: String,
}

impl<T> PartialEq for SelectProps<T>
where
    T: Clone + PartialEq + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.options, &other.options)
            // && Arc::ptr_eq(&self.filter, &other.filter) // TODO: don't ignore filter changes?
            && self.omit_selected == other.omit_selected
            && self.placeholder == other.placeholder
            && self.onselected == other.onselected
            && self.onremoved == other.onremoved
    }
}

pub enum Msg {
    Noop,

    Input(String),
    ClearSearch,
    Filtered,

    Selected(usize),
    Removed(usize),
    Hover(usize),

    Focus,
    Blur,
    KeyPress(KeyboardEvent),
}

impl<T> Component for Select<T>
where
    T: Clone + PartialEq + Display + 'static,
{
    type Properties = SelectProps<T>;
    type Message = Msg;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            focused: false,
            selection_index: 0,
            search_text: String::new(),
            props,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Noop => false,

            Msg::Filtered => true,

            Msg::Input(input) => {
                self.focused = true;
                self.search_text = input.clone();

                let filter_fn = self.props.filter.clone();
                let options = self.props.options.clone();
                self.link.send_future(async move {
                    if input.is_empty() {
                        options.unfilter().await;
                    } else {
                        options.filter(|item| (filter_fn)(item, &input)).await;
                    }
                    Msg::Filtered
                });
                true
            }

            Msg::ClearSearch => {
                let options = self.props.options.clone();
                self.link.send_future(async move {
                    options.unfilter().await;
                    Msg::Filtered
                });
                self.search_text.clear();
                true
            }

            Msg::Selected(idx) => {
                if let Some(ref onselected) = self.props.onselected {
                    onselected.emit(idx);
                }
                self.link
                    .send_message_batch(vec![Msg::ClearSearch, Msg::Blur]);
                false
            }

            Msg::Removed(idx) => {
                if let Some(ref onremoved) = self.props.onremoved {
                    onremoved.emit(idx);
                }
                false
            }

            Msg::Hover(idx) => {
                self.selection_index = idx;
                true
            }

            Msg::Focus => {
                self.focused = true;
                true
            }

            Msg::Blur => {
                self.focused = false;
                self.selection_index = 0;
                self.search_text.clear();
                true
            }

            Msg::KeyPress(event) => match event.code().as_ref() {
                "Enter" => {
                    if let Some((index, _)) = self.props.options.get_filtered(self.selection_index)
                    {
                        self.link.send_message(Msg::Selected(index));
                    }
                    false
                }

                "ArrowUp" => {
                    self.focused = true;
                    let event: &Event = &event;
                    event.prevent_default();
                    self.selection_index = self.selection_index.saturating_sub(1);
                    true
                }

                "ArrowDown" => {
                    self.focused = true;
                    let event: &Event = &event;
                    event.prevent_default();
                    self.selection_index += 1;
                    true
                }

                _ => false,
            },
        }
    }

    fn view(&self) -> Html {
        // let filtered_items = if let Ok(items) = self.props.options.inner.read() {
        //     match self.filtered_indices {
        //         FilteredOptions::All => items.iter().cloned().enumerate().collect(),
        //         FilteredOptions::Some(ref indices) => {
        //             let mut filtered = Vec::with_capacity(indices.len());
        //             for index in indices.iter() {
        //                 if let Some(item) = items.get(*index) {
        //                     filtered.push((*index, item.clone()))
        //                 }
        //             }
        //             filtered
        //         }
        //         FilteredOptions::None => Vec::new(),
        //     }
        // } else {
        //     Vec::new()
        // };

        let options = if self.props.omit_selected {
            self.props
                .options
                .filtered_items()
                .into_iter()
                .filter(|(_, selected, _)| !selected)
                .collect::<Vec<_>>()
        } else {
            self.props.options.filtered_items()
        };

        let options = if options.is_empty() {
            html! {
                <div class="has-text-centered">
                    <p>
                        <span class="icon">
                            <i class="fas fa-inbox" />
                        </span>
                    </p>
                    <p>{"No Data"}</p>
                </div>
            }
        } else {
            options
                .into_iter()
                .enumerate()
                .map(|(i, (idx, selected, item))| {
                    html! {
                        <a
                            class=classes!(
                                "dropdown-item",
                                if self.selection_index == i {"is-active"}
                                else if selected {"has-background-primary-light"}
                                else {""}
                            )
                        >
                            <p
                                onmouseenter=self.link.callback(move |_| Msg::Hover(i))
                                onmousedown=self.link.callback(move |event: MouseEvent| {
                                    let event: &Event = &event;
                                    event.prevent_default();
                                    Msg::Selected(idx)
                                })
                            >
                                {item.to_string()}
                            </p>
                        </a>
                    }
                })
                .collect::<Html>()
        };

        html! {
            <div class=classes!("dropdown", if self.focused {"is-active"} else {""})>
                <div class="dropdown-trigger">
                {
                    if self.props.options.is_multiple() {
                        self.view_multiple()
                    } else {
                        self.view_single()
                    }
                }
                </div>
                <div class="dropdown-menu">
                    <div class="dropdown-content">
                        { options }
                    </div>
                </div>
            </div>
        }
    }
}

impl<T> Select<T>
where
    T: Clone + PartialEq + Display + 'static,
{
    fn view_single(&self) -> Html {
        if self.focused {
            html! {
                <div class="control has-icons-right">
                    <input
                        class="input"
                        type="text"
                        value=&self.search_text
                        placeholder=self.props.options.selected_items().first().map(|(_, x)| x.to_string()).unwrap_or_else(|| self.props.placeholder.clone())
                        oninput=self.link.callback(|event: InputData| Msg::Input(event.value))
                        onfocus=self.link.callback(|_| Msg::Focus)
                        onblur=self.link.callback(|_| Msg::Blur)
                        onkeydown=self.link.callback(Msg::KeyPress)
                    />
                    <span class="icon is-small is-right">
                    {
                        if self.search_text.is_empty() {
                            html! { <i class="fas fa-search" /> }
                        } else {
                            html! {<button class="delete" onclick=self.link.callback(|_| Msg::ClearSearch) /> }
                        }
                    }
                    </span>
                </div>
            }
        } else {
            html! {
                <div class="control has-icons-right">
                    <input
                        class="input"
                        type="text"
                        value=self.props.options.selected_items().first().map(|(_, x)| x.to_string()).unwrap_or_default()
                        oninput=self.link.callback(|data: InputData| {
                            // Don't allow input when not focused
                            let event: &Event = &data.event;
                            event.prevent_default();
                            Msg::Focus
                        })
                        onfocus=self.link.callback(|_| Msg::Focus)
                        onblur=self.link.callback(|_| Msg::Blur)
                        onclick=self.link.callback(|_| Msg::Focus)
                        onkeydown=self.link.callback(Msg::KeyPress)
                    />
                    <span class="icon is-small is-right">
                        <i class="fas fa-angle-down" />
                    </span>
                </div>
            }
        }
    }

    fn view_multiple(&self) -> Html {
        html! {
            <div class=classes!("input", "ybss-multiple-input-wrapper", if self.focused {"is-active"} else {""})>
                {
                    for self.props.options.selected_items().into_iter().map(|(i, item)| html! {
                        <span class="tag">
                            {item.to_string()}
                            <div class="delete is-small" onclick=self.link.callback(move |_| Msg::Removed(i)) />
                        </span>
                    })
                }
                <input
                    class="input"
                    type="text"
                    placeholder="Type to search"
                    value=&self.search_text
                    oninput=self.link.callback(|event: InputData| Msg::Input(event.value))
                    onfocus=self.link.callback(|_| Msg::Focus)
                    onblur=self.link.callback(|_| Msg::Blur)
                    onkeydown=self.link.callback(Msg::KeyPress)
                />
            </div>
        }
    }
}
