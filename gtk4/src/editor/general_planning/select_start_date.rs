use chrono::Datelike;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use libadwaita::glib::TimeZone;
use relm4::gtk::prelude::OrientableExt;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    start_date: collomatique_time::WeekStart,
    current_selected_date: chrono::NaiveDate,
    update_date: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(collomatique_time::WeekStart),
    Cancel,
    Accept,
    Select(chrono::NaiveDate),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_time::WeekStart),
}

impl Dialog {
    fn glib_date_from_chrono(date: &chrono::NaiveDate) -> gtk::glib::DateTime {
        use chrono::Datelike;
        gtk::glib::DateTime::new(
            &TimeZone::local(),
            date.year(),
            date.month() as i32,
            date.day() as i32,
            0,
            0,
            0.,
        )
        .expect("valid date")
    }

    fn chrono_date_from_glib(date: &gtk::glib::DateTime) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(
            date.year(),
            date.month() as u32,
            date.day_of_month() as u32,
        )
        .expect("valid date")
    }

    fn generate_selected_date_text(&self) -> String {
        format!(
            "Date sélectionnée: {}",
            self.start_date.monday().format("%d/%m/%Y")
        )
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: false,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Début des colles"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    set_margin_all: 5,
                    #[name(calendar)]
                    gtk::Calendar {
                        #[track(model.update_date)]
                        select_day: &Self::glib_date_from_chrono(&model.current_selected_date),
                        connect_day_selected[sender] => move |widget| {
                            let current_date = widget.date();
                            sender.input(DialogInput::Select(
                                Self::chrono_date_from_glib(&current_date)
                            ))
                        },
                        connect_next_month[sender] => move |widget| {
                            let current_date = widget.date();
                            sender.input(DialogInput::Select(
                                Self::chrono_date_from_glib(&current_date)
                            ))
                        },
                        connect_prev_month[sender] => move |widget| {
                            let current_date = widget.date();
                            sender.input(DialogInput::Select(
                                Self::chrono_date_from_glib(&current_date)
                            ))
                        },
                        connect_next_year[sender] => move |widget| {
                            let current_date = widget.date();
                            sender.input(DialogInput::Select(
                                Self::chrono_date_from_glib(&current_date)
                            ))
                        },
                        connect_prev_year[sender] => move |widget| {
                            let current_date = widget.date();
                            sender.input(DialogInput::Select(
                                Self::chrono_date_from_glib(&current_date)
                            ))
                        },
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &model.generate_selected_date_text(),
                    },
                }
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            start_date: collomatique_time::WeekStart::new(
                chrono::NaiveDate::from_ymd_opt(2025, 09, 01).unwrap(),
            )
            .unwrap(),
            current_selected_date: chrono::NaiveDate::from_ymd_opt(2025, 09, 01).unwrap(),
            update_date: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.update_date = false;
        match msg {
            DialogInput::Show(date) => {
                self.hidden = false;
                self.start_date = date;
                self.current_selected_date = self.start_date.monday().clone();
                self.update_date = true;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.start_date.clone()))
                    .unwrap();
            }
            DialogInput::Select(date) => {
                self.start_date = collomatique_time::WeekStart::round_from(date.clone());
                self.current_selected_date = date;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.calendar.clear_marks();
        if self.start_date.monday().month0() == self.current_selected_date.month0() {
            widgets.calendar.mark_day(self.start_date.monday().day());
        }
    }
}
