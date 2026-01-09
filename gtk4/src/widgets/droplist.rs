use gtk::prelude::WidgetExt;
use relm4::gtk::glib;
use relm4::{gtk, Component};
use relm4::{ComponentParts, ComponentSender};

#[derive(Debug)]
pub struct WidgetParams {
    pub initial_list: Vec<String>,
    pub initial_selected: Option<usize>,
    pub enable_search: bool,
    pub width_request: i32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum WidgetInput {
    UpdateList(Vec<String>, Option<usize>),
    ForceSelect(usize),

    SelectionChanged(u32),
}

#[derive(Debug)]
pub enum WidgetOutput {
    SelectionChanged(Option<usize>),
}

pub struct Widget {
    current_list: Vec<String>,
    update_list: bool,
    currently_selected: Option<u32>,
    update_selected: bool,
    enable_search: bool,
    width_request: i32,
    gtk_model: gtk::StringList,
    gtk_expression: gtk::Expression,
}

#[relm4::component(pub)]
impl Component for Widget {
    type Input = WidgetInput;
    type Output = WidgetOutput;
    type Init = WidgetParams;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::DropDown {
            set_size_request: (model.width_request, -1),
            #[track(model.update_list)]
            set_model: Some(&model.gtk_model),
            #[track(model.update_selected)]
            set_selected: match model.currently_selected {
                Some(x) => x,
                None => gtk::INVALID_LIST_POSITION,
            },
            set_enable_search: model.enable_search,
            set_expression: Some(&model.gtk_expression),
            connect_selected_notify[sender] => move |widget| {
                sender.input(
                    WidgetInput::SelectionChanged(
                        widget.selected()
                    )
                );
            },
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        assert!(params.initial_list.len() < u32::MAX as usize);
        if let Some(num) = params.initial_selected {
            assert!(num < params.initial_list.len());
        }

        use gtk::glib::closure;
        let gtk_expression = gtk::ClosureExpression::new::<String>(
            &[] as &[gtk::Expression],
            closure!(|item: Option<gtk::StringObject>| {
                match item {
                    None => String::new(),
                    Some(it) => it.string().to_string(),
                }
            }),
        );

        let mut model = Widget {
            current_list: params.initial_list,
            update_list: false,
            currently_selected: params.initial_selected.map(|x| x as u32),
            update_selected: false,
            width_request: params.width_request,
            enable_search: params.enable_search,
            gtk_model: gtk::StringList::new(&[]),
            gtk_expression: gtk_expression.upcast(),
        };
        model.update_gtk_model();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.update_selected = false;
        self.update_list = false;
        match message {
            WidgetInput::UpdateList(new_list, selected) => {
                assert!(new_list.len() < u32::MAX as usize);
                self.current_list = new_list;
                self.update_gtk_model();
                if let Some(num) = selected {
                    if num >= self.current_list.len() {
                        return;
                    }
                }

                self.currently_selected = selected.map(|x| {
                    assert!(x < u32::MAX as usize);
                    x as u32
                });
                self.update_selected = true;
            }
            WidgetInput::ForceSelect(selected) => {
                assert!(selected < self.current_list.len());
                let selected_u32 = selected as u32;
                if Some(selected_u32) == self.currently_selected {
                    return;
                }
                self.currently_selected = Some(selected_u32);
                self.update_selected = true;
            }
            WidgetInput::SelectionChanged(selected) => {
                if Some(selected) == self.currently_selected {
                    return;
                }
                if selected == gtk::INVALID_LIST_POSITION {
                    self.currently_selected = None;
                    sender.output(WidgetOutput::SelectionChanged(None)).unwrap();
                } else {
                    self.currently_selected = Some(selected);
                    let selected_usize = selected as usize;
                    sender
                        .output(WidgetOutput::SelectionChanged(Some(selected_usize)))
                        .unwrap();
                }
            }
        }
    }
}

impl Widget {
    fn update_gtk_model(&mut self) {
        let string_list: Vec<&str> = self.current_list.iter().map(|x| x.as_str()).collect();
        self.gtk_model = gtk::StringList::new(&string_list);
        self.update_list = true;
    }
}
