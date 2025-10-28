use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_ops::SettingsUpdateOp;

#[derive(Debug)]
pub enum SettingsInput {
    Update(
        collomatique_state_colloscopes::students::Students<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::settings::Settings<
            collomatique_state_colloscopes::StudentId,
        >,
    ),
}

pub struct Settings {
    students: collomatique_state_colloscopes::students::Students<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    >,
    settings: collomatique_state_colloscopes::settings::Settings<
        collomatique_state_colloscopes::StudentId,
    >,
}

#[relm4::component(pub)]
impl Component for Settings {
    type Input = SettingsInput;
    type Output = SettingsUpdateOp;
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
                gtk::Label {
                    set_label: "<b>En construction...</b>",
                    set_use_markup: true,
                }
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Settings {
            students: collomatique_state_colloscopes::students::Students::default(),
            settings: collomatique_state_colloscopes::settings::Settings::default(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SettingsInput::Update(students, settings) => {
                self.students = students;
                self.settings = settings;
            }
        }
    }
}
