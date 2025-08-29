use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

mod subjects_display;

#[derive(Debug)]
pub enum SubjectsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::subjects::Subjects,
    ),
    AddSubjectClicked,
}

pub struct Subjects {
    periods: collomatique_state_colloscopes::periods::Periods,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    subjects_list: FactoryVecDeque<subjects_display::Entry>,
}

#[relm4::component(pub)]
impl Component for Subjects {
    type Input = SubjectsInput;
    type Output = ();
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_margin_all: 5,
            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                #[local_ref]
                subjects_box -> gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 20,
                    set_spacing: 30,
                    #[watch]
                    set_visible: !model.subjects.ordered_subject_list.is_empty(),
                },
                gtk::Button {
                    set_margin_top: 10,
                    connect_clicked => SubjectsInput::AddSubjectClicked,
                    adw::ButtonContent {
                        set_icon_name: "edit-add",
                        set_label: "Ajouter une matière",
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let subjects_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();

        let model = Subjects {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            subjects_list,
        };
        let subjects_box = model.subjects_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SubjectsInput::Update(new_periods, new_subjects) => {
                self.periods = new_periods;
                self.subjects = new_subjects;

                crate::tools::factories::update_vec_deque(
                    &mut self.subjects_list,
                    self.subjects.ordered_subject_list.iter().map(|(id, desc)| {
                        subjects_display::EntryData {
                            subject_params: desc.parameters.clone(),
                            global_first_week: self.periods.first_week.clone(),
                            periods: self
                                .periods
                                .ordered_period_list
                                .iter()
                                .map(|(id, period_desc)| subjects_display::PeriodData {
                                    week_count: period_desc.len(),
                                    status: !desc.excluded_periods.contains(id),
                                })
                                .collect(),
                            subject_id: id.clone(),
                            subject_count: self.subjects.ordered_subject_list.len(),
                        }
                    }),
                    |data| subjects_display::EntryInput::UpdateData(data),
                );
            }
            SubjectsInput::AddSubjectClicked => {}
        }
    }
}
