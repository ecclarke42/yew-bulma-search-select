use yew::prelude::*;

use yew_bulma_search_select::{filter, Select, SelectFilter, SelectOptions, Selection};

fn main() {
    yew::start_app::<App>();
}

/// Test data struct
#[derive(Clone, PartialEq)]
pub struct Data {
    name: String,
    value: u32,
}

impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Simple yew application that spawns a notification
pub struct App {
    link: ComponentLink<Self>,

    filter: SelectFilter<Data>,

    a_data: SelectOptions<Data>,
    b_data: SelectOptions<Data>,
    c_data: SelectOptions<Data>,
}

pub enum Msg {
    SelectedA(usize),
    ClearedA(usize),

    SelectedB(usize),
    ClearedB(usize),

    SelectedC(usize),
    ClearedC(usize),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let test_data = vec![
            Data {
                name: String::from("First"),
                value: 0,
            },
            Data {
                name: String::from("Second"),
                value: 1,
            },
            Data {
                name: String::from("Third"),
                value: 2,
            },
            Data {
                name: String::from("Fourth"),
                value: 3,
            },
            Data {
                name: String::from("Fifth"),
                value: 4,
            },
            Data {
                name: String::from("Something else with \"first\""),
                value: 5,
            },
        ];

        Self {
            link,
            filter: filter(|item: &Data, search: &str| -> bool {
                item.name
                    .to_lowercase()
                    .find(&search.to_lowercase())
                    .is_some()
            }),
            a_data: Selection::one(0).with_options(test_data.clone()),
            b_data: Selection::none().with_options(test_data.clone()),
            c_data: Selection::empty().with_options(test_data),
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SelectedA(index) => self.a_data.select(index),
            Msg::ClearedA(index) => self.a_data.deselect(index),

            Msg::SelectedB(index) => self.b_data.select(index),
            Msg::ClearedB(index) => self.b_data.deselect(index),

            Msg::SelectedC(index) => self.c_data.select(index),
            Msg::ClearedC(index) => self.c_data.deselect(index),
        }
    }

    fn view(&self) -> Html {
        html! {
            <main>
                <div class="field">
                    <label class="label">{"Select Single, Non-Nullable Field"}</label>
                    <div class="control">
                        <Select<Data>
                            options=self.a_data.clone()
                            filter=self.filter.clone()
                            onselected=self.link.callback(Msg::SelectedA)
                        />
                    </div>
                </div>
                <div class="field">
                    <label class="label">{"Select Single, Nullable Field"}</label>
                    <div class="control">
                        <Select<Data>
                            options=self.b_data.clone()
                            filter=self.filter.clone()
                            onselected=self.link.callback(Msg::SelectedB)
                        />
                    </div>
                </div>
                <div class="field">
                    <label class="label">{"Select Multiple Fields"}</label>
                    <div class="control">
                        <Select<Data>
                            options=self.c_data.clone()
                            filter=self.filter.clone()
                            onselected=self.link.callback(Msg::SelectedC)
                            onremoved=self.link.callback(Msg::ClearedC)
                        />
                    </div>
                </div>
                <div class="field">
                    <label class="label">{"Select Multiple Fields (Clone, omit selections from options)"}</label>
                    <div class="control">
                        <Select<Data>
                            omit_selected={true}
                            options=self.c_data.clone()
                            filter=self.filter.clone()
                            onselected=self.link.callback(Msg::SelectedC)
                            onremoved=self.link.callback(Msg::ClearedC)
                        />
                    </div>
                </div>
                <div class="field">
                    <label class="label">{"Dropdown for comparison"}</label>
                    <div class="control">
                        <div class="dropdown">
                            <div class="dropdown-trigger">
                                <button class="button">
                                    <span>{"Dropdown"}</span>
                                    <span class="icon is-small">
                                        <i class="fas fa-angle-down" />
                                    </span>
                                </button>
                            </div>
                            <div class="dropdown-menu">
                            </div>
                        </div>
                    </div>
                </div>
            </main>
        }
    }
}
