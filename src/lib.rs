use yew::prelude::*;
use yewtil::future::LinkFuture;

mod state;
pub use state::SelectState;
mod selection;
pub use selection::Selection;
mod wrappers;
pub use wrappers::{SelectDisplay, SelectFilter};

/// Bulma-based selection box
/// TODO: document
pub struct Select<T: 'static> {
    link: ComponentLink<Self>,
    props: SelectProps<T>,

    focused: bool,
    selection_index: usize,
    search_text: String,
}

#[derive(Properties)]
pub struct SelectProps<T> {
    /// Omit selected items from the dropdown list (if false, selected will be
    /// highlighted).
    #[prop_or_default]
    pub omit_selected: bool,

    /// Control whether to show the selected items as tags in the input field
    /// (if in multiple mode). Useful to turn off if you want to control display
    /// of the selected items outside of the field.
    ///
    /// This does nothing in single selection mode.
    #[prop_or(true)]
    pub display_selected: bool,

    pub state: SelectState<T>,
    pub display: SelectDisplay<T>,

    #[prop_or_default]
    pub onselected: Option<Callback<usize>>,
    #[prop_or_default]
    pub onremoved: Option<Callback<usize>>,

    #[prop_or_else(|| String::from("Type to search"))]
    pub placeholder: String,
    #[prop_or_default]
    pub readonly: bool,
    #[prop_or_default]
    pub disabled: bool,
    #[prop_or_default]
    pub loading: bool,
}

// This SHOULD be the auto impl, but for some reason that thinks that T needs to be Clone
impl<T> Clone for SelectProps<T> {
    fn clone(&self) -> Self {
        Self {
            omit_selected: self.omit_selected,
            display_selected: self.display_selected,

            state: self.state.clone(),
            display: self.display.clone(),

            onselected: self.onselected.clone(),
            onremoved: self.onremoved.clone(),

            placeholder: self.placeholder.clone(),
            readonly: self.readonly,
            disabled: self.disabled,
            loading: self.loading,
        }
    }
}

impl<T> PartialEq for SelectProps<T> {
    fn eq(&self, other: &Self) -> bool {
        self.readonly == other.readonly && self.disabled == other.disabled && self.loading == other.loading &&
            self.state == other.state
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

impl<T: 'static> Component for Select<T> {
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
            if props.disabled {
                self.focused = false;
                self.selection_index = 0;
                self.search_text.clear();
            }
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
                if self.props.disabled || self.props.readonly {
                    return false;
                }

                self.focused = true;
                self.search_text = input.clone();

                let state = self.props.state.clone();
                self.link.send_future(async move {
                    if input.is_empty() {
                        state.unfilter().await;
                    } else {
                        state.filter(&input).await;
                    }
                    Msg::Filtered
                });
                true
            }

            Msg::ClearSearch => {
                let options = self.props.state.clone();
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
                if self.props.disabled || self.props.readonly {
                    return false;
                }
                self.focused = true;
                true
            }

            Msg::Blur => {
                self.focused = false;
                self.selection_index = 0;
                self.search_text.clear();
                true
            }

            Msg::KeyPress(event) => {
                if self.props.disabled || self.props.readonly {
                    return false;
                }
                match event.code().as_ref() {
                    "Enter" => {
                        if let Some((index, _)) =
                            self.props.state.get_filtered(self.selection_index)
                        {
                            self.link.send_message(Msg::Selected(index));
                        }
                        false
                    }

                    "Escape" => {
                        self.focused = false;
                        self.selection_index = 0;
                        self.search_text.clear();
                        true
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
                }
            }
        }
    }

    fn view(&self) -> Html {
        let options = if self.props.omit_selected {
            self.props
                .state
                .filtered_items()
                .into_iter()
                .filter(|(_, selected, _)| !selected)
                .collect::<Vec<_>>()
        } else {
            self.props.state.filtered_items()
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
                                { self.props.display.call(&item) }
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
                    if self.props.state.is_multiple() {
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

impl<T> Select<T> {
    fn view_single(&self) -> Html {
        if self.focused {
            html! {
                <div class="control has-icons-right">
                    <input
                        class=classes!("input", if self.props.loading {"is-loading"} else {""})
                        type="text"
                        value=&self.search_text
                        placeholder=self.props.state.selected_items().first().map(|(_, x)| self.props.display.call(x)).unwrap_or_else(|| self.props.placeholder.clone())
                        oninput=self.link.callback(|event: InputData| Msg::Input(event.value))
                        onfocus=self.link.callback(|_| Msg::Focus)
                        onblur=self.link.callback(|_| Msg::Blur)
                        onkeydown=self.link.callback(Msg::KeyPress)
                        disabled=self.props.disabled
                        readonly=self.props.readonly
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
                        class=classes!("input", if self.props.loading {"is-loading"} else {""})
                        type="text"
                        value=self.props.state.selected_items().first().map(|(_, x)| self.props.display.call(x)).unwrap_or_default()
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
                        disabled=self.props.disabled
                        readonly=self.props.readonly
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
                    if self.props.display_selected {
                        self.props.state.selected_items().into_iter().map(|(i, item)| html! {
                            <span class="tag">
                                { self.props.display.call(&item) }
                                <div class="delete is-small" onclick=self.link.callback(move |_| Msg::Removed(i)) />
                            </span>
                        }).collect::<Html>()
                    } else {
                        html! {}
                    }
                }
                <input
                    class=classes!("input", if self.props.loading {"is-loading"} else {""})
                    type="text"
                    placeholder="Type to search"
                    value=&self.search_text
                    oninput=self.link.callback(|event: InputData| Msg::Input(event.value))
                    onfocus=self.link.callback(|_| Msg::Focus)
                    onblur=self.link.callback(|_| Msg::Blur)
                    onkeydown=self.link.callback(Msg::KeyPress)
                    disabled=self.props.disabled
                    readonly=self.props.readonly
                />
            </div>
        }
    }
}
