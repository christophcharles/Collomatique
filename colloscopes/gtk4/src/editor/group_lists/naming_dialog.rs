mod result_display;
mod spec_row;

use std::collections::BTreeSet;

use adw::prelude::PreferencesGroupExt;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};

use collomatique_greedy_groups::{
    GenerationRequest, build_generation_plan, greedy_group_lists_with_log,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::{PeriodId, SubjectId};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

/// One generated list: the sealed group list and the `(period, subject)` pairs it must be
/// associated to — the payload `GroupListsUpdateOp::AddGeneratedGroupLists` takes.
type GeneratedList = (GroupList, BTreeSet<(PeriodId, SubjectId)>);

/// Modal naming dialog: the second step of the generation chain, and the one that actually
/// generates. It turns the configured request into a
/// [`GenerationPlan`](collomatique_greedy_groups::GenerationPlan), runs the greedy off the
/// UI thread while streaming its log into a [`DebugView`], shows one editable name per spec and
/// the resulting groups, and reports the pairs nobody is registered for. "Valider" lands the
/// greedy answer as it stands — it is the whole result.
pub struct Dialog {
    hidden: bool,
    move_front: bool,
    /// Toggles the content between the naming rows and the greedy log.
    show_debug: bool,
    /// Flips the header indicator from spinner to "ok". Distinct from `generated.is_some()`:
    /// with zero specs there is nothing to generate, so the indicator shows "done" while
    /// "Valider" stays insensitive.
    done: bool,
    /// Discards results from a superseded `Show` (or from after a cancel).
    build_seq: u64,
    /// The parameters the plan was built against: the student names of the result display come
    /// from them.
    params: Parameters,
    /// `Some` once the off-thread greedy has answered, its lists in plan order. Consumed by
    /// "Valider", which renames them and hands them over.
    outcome: Option<Vec<GeneratedList>>,
    /// The skipped-pairs warning; empty when nothing was skipped.
    skipped_text: String,
    /// What each spec covers, in plan/spec order. The naming rows seed their name from it and
    /// the result display titles its expanders with it, so the two views read as one list even
    /// after the user has renamed everything.
    coverages: Vec<String>,
    /// Mirror of the naming rows' current titles and names, in plan/spec order.
    rows_data: Vec<spec_row::Data>,
    rows: FactoryVecDeque<spec_row::SpecRow>,
    /// The read-only view of the generated groups, in the same order.
    results_data: Vec<result_display::Data>,
    results: FactoryVecDeque<result_display::ListRow>,
    debug_view: Controller<DebugView>,
}

impl Dialog {
    /// Whether there is a generated result to hand over. Zero specs never produce one, which is
    /// what keeps "Valider" insensitive in that case.
    fn has_result(&self) -> bool {
        self.outcome.is_some()
    }

    /// The names the user currently has in the naming rows, in plan order.
    fn names(&self) -> Vec<String> {
        self.rows_data.iter().map(|d| d.name.clone()).collect()
    }
}

#[derive(Debug)]
pub enum DialogInput {
    /// Open the dialog for this request, against the parameters the config dialog echoed back.
    Show(GenerationRequest, Parameters),
    Cancel,
    Accept,
    /// One log line, streamed from the off-thread greedy.
    Echo(String),
    ToggleDebug(bool),
    /// (spec index, new name), forwarded from a row.
    SetName(usize, String),
}

#[derive(Debug)]
pub enum DialogOutput {
    /// The generated lists, named as the user left them: the op payload itself.
    Accepted(Vec<GeneratedList>),
    Cancelled,
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[derive(Debug)]
pub enum DialogCommandOutput {
    /// (the `build_seq` this run was spawned with, what the greedy produced)
    Generated(u64, Vec<GeneratedList>),
}

/// The skipped-pairs warning text; empty when nothing was skipped.
fn skipped_label(params: &Parameters, skipped: &BTreeSet<(PeriodId, SubjectId)>) -> String {
    if skipped.is_empty() {
        return String::new();
    }

    let pairs: Vec<String> = skipped
        .iter()
        .map(|&(period, subject)| {
            let name = params
                .subjects
                .find_subject(subject)
                .expect("the skipped pair comes from a plan built against these parameters")
                .parameters
                .name
                .clone();
            let position = params
                .periods
                .find_period_position(period)
                .expect("the skipped pair comes from a plan built against these parameters");
            format!("{} (période {})", name, position + 1)
        })
        .collect();

    format!(
        "Aucun élève n'est inscrit pour : {}. Ces listes ne seront pas générées.",
        pairs.join(", ")
    )
}

/// The generated lists as the result display shows them, in the same order. Each expander is
/// titled by what its spec covers, not by the list's name: the name is the naming row's
/// business, right above, and it changes under the user's hands.
fn result_data(
    params: &Parameters,
    coverages: &[String],
    generated: &[GeneratedList],
) -> Vec<result_display::Data> {
    generated
        .iter()
        .zip(coverages)
        .map(|((list, _covered), coverage)| {
            let groups: Vec<result_display::GroupData> = match list.filling() {
                collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                    groups,
                } => groups
                    .iter()
                    .enumerate()
                    .map(|(index, group)| {
                        let mut students: Vec<String> = group
                            .students
                            .iter()
                            .map(|&id| {
                                collomatique_ui_text::rendering::render_student(
                                    &params.students,
                                    id,
                                )
                                .expect(
                                    "the student comes from a plan built against these parameters",
                                )
                            })
                            .collect();
                        // Set order is by id, which is meaningless to the reader.
                        students.sort();
                        result_display::GroupData {
                            title: format!("Groupe {}", index + 1),
                            students: students.join(", "),
                        }
                    })
                    .collect(),
                // The generator only ever produces prefilled lists.
                collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                    ..
                } => Vec::new(),
            };

            let students = list.filling().iter_students().count();
            let subtitle = format!(
                "{} groupe{}, {} élève{}",
                groups.len(),
                if groups.len() > 1 { "s" } else { "" },
                students,
                if students > 1 { "s" } else { "" },
            );

            result_display::Data {
                title: coverage.clone(),
                subtitle,
                groups,
            }
        })
        .collect()
}

/// The generated lists with the user's names on them. Only the name changes, so the prefilled
/// invariants — which do not mention it — still hold.
fn rename(generated: &[GeneratedList], names: &[String]) -> Vec<GeneratedList> {
    generated
        .iter()
        .zip(names)
        .map(|((list, covered), name)| {
            let (mut params, filling) = list.clone().into_parts();
            params.name = name.clone();
            let list =
                GroupList::new(params, filling).expect("renaming a valid list leaves it valid");
            (list, covered.clone())
        })
        .collect()
}

#[relm4::component(pub)]
impl Component for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;
    type CommandOutput = DialogCommandOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Nommer les nouvelles listes de groupes"),
            set_default_size: (700, 650),
            connect_close_request[sender] => move |_| {
                sender.input(DialogInput::Cancel);
                gtk::glib::Propagation::Stop
            },
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
                        // Nothing to validate until the greedy has answered — and with no spec
                        // at all, nothing is ever generated.
                        #[watch]
                        set_sensitive: model.has_result(),
                        connect_clicked => DialogInput::Accept,
                    },
                    // Packed after "Valider", so it lands to its left.
                    #[name(terminal_toggle)]
                    pack_end = &gtk::ToggleButton {
                        set_tooltip: "Afficher/Cacher la sortie de débogage",
                        // Block the `toggled` handler while we set `active` programmatically:
                        // otherwise the setter re-emits `toggled`, which re-sends `ToggleDebug`,
                        // which sets `active` again — an infinite loop under rapid clicking.
                        #[track(terminal_toggle.is_active() != model.show_debug)]
                        #[block_signal(toggled_handler)]
                        set_active: model.show_debug,
                        connect_toggled[sender] => move |btn| {
                            sender.input(DialogInput::ToggleDebug(btn.is_active()));
                        } @toggled_handler,
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::Center,
                            adw::Spinner {
                                #[watch]
                                set_visible: !model.done,
                            },
                            gtk::Image::from_icon_name("object-select-symbolic") {
                                #[watch]
                                set_visible: model.done,
                            },
                        },
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 10,
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        #[watch]
                        set_visible: !model.skipped_text.is_empty(),
                        gtk::Image::from_icon_name("dialog-warning-symbolic") {
                            set_valign: gtk::Align::Start,
                        },
                        gtk::Label {
                            add_css_class: "warning",
                            set_hexpand: true,
                            set_wrap: true,
                            set_xalign: 0.,
                            #[watch]
                            set_label: &model.skipped_text,
                        },
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        #[watch]
                        set_visible: !model.show_debug,
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 10,
                            #[local_ref]
                            rows_box -> adw::PreferencesGroup {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_title: "Noms des listes générées",
                            },
                            adw::Spinner {
                                set_halign: gtk::Align::Center,
                                set_margin_all: 15,
                                set_size_request: (48, 48),
                                #[watch]
                                set_visible: !model.done,
                            },
                            #[local_ref]
                            results_box -> adw::PreferencesGroup {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_title: "Répartition proposée",
                                #[watch]
                                set_visible: model.has_result(),
                            },
                            gtk::Box {
                                set_vexpand: true,
                            },
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        #[watch]
                        set_visible: model.show_debug,
                        append: model.debug_view.widget(),
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let rows = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                spec_row::SpecRowOutput::NameChanged(index, name) => {
                    DialogInput::SetName(index, name)
                }
            });

        let results = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .detach();

        let debug_view = DebugView::builder().launch(None).detach();

        let model = Dialog {
            hidden: true,
            move_front: false,
            show_debug: false,
            done: false,
            build_seq: 0,
            params: Parameters::default(),
            outcome: None,
            skipped_text: String::new(),
            coverages: Vec::new(),
            rows_data: Vec::new(),
            rows,
            results_data: Vec::new(),
            results,
            debug_view,
        };

        let rows_box = model.rows.widget();
        let results_box = model.results.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.move_front = false;
        match msg {
            DialogInput::Show(request, params) => {
                self.hidden = false;
                self.move_front = true;
                self.show_debug = false;
                self.done = false;
                self.outcome = None;
                // Any run still going from a previous opening is now stale.
                self.build_seq += 1;
                self.debug_view.emit(DebugViewInput::Clear);

                self.params = params;

                // The config dialog only offers valid choices, and both dialogs are modal, so
                // the document cannot have changed in between: a plan error here is a caller
                // bug (that is exactly what `GenerationPlanError` documents).
                let plan = build_generation_plan(&self.params, &request)
                    .expect("the config dialog assembled the request against these parameters");

                self.skipped_text = skipped_label(&self.params, &plan.skipped);
                self.coverages = plan
                    .specs
                    .iter()
                    .map(|(_spec, covered)| {
                        collomatique_ui_text::rendering::coverage_label(
                            &self.params.periods,
                            &self.params.subjects,
                            covered,
                        )
                    })
                    .collect();
                self.rows_data = self
                    .coverages
                    .iter()
                    .map(|coverage| spec_row::Data {
                        title: format!("Liste pour {}", coverage),
                        name: coverage.clone(),
                    })
                    .collect();
                crate::tools::factories::update_vec_deque(
                    &mut self.rows,
                    self.rows_data.iter().cloned(),
                    spec_row::SpecRowInput::UpdateData,
                );
                self.results_data.clear();
                self.refresh_results();

                if plan.specs.is_empty() {
                    // Every selected pair was skipped: nothing to name and nothing to generate.
                    // The dialog still opens — it is where the user learns why — but "Valider"
                    // stays insensitive, since no result will ever exist.
                    self.done = true;
                } else {
                    // The greedy is fast but not instant, and it is the whole answer: run it off
                    // the UI thread and stream its log back as `Echo`, which lands live in the
                    // DebugView. The names it is given are the defaults; the user's edits are
                    // patched in afterwards, so an edit never restarts the run.
                    let seq = self.build_seq;
                    let names = self.names();
                    let input = sender.input_sender().clone();
                    sender.spawn_oneshot_command(move || {
                        let mut log = move |line: &str| {
                            input.emit(DialogInput::Echo(format!("{}\n", line)));
                        };
                        let outcome = greedy_group_lists_with_log(&plan, &names, &mut log);
                        DialogCommandOutput::Generated(seq, outcome)
                    });
                }
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender.output(DialogOutput::Cancelled).unwrap();
                }
            }
            DialogInput::Accept => {
                // "Valider" is insensitive until the result exists, so a missing one here can
                // only come from a race; ignore it.
                let Some(outcome) = self.outcome.take() else {
                    return;
                };
                let entries = rename(&outcome, &self.names());

                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
                sender.output(DialogOutput::Accepted(entries)).unwrap();
            }
            DialogInput::Echo(line) => {
                self.debug_view.emit(DebugViewInput::Append(line));
            }
            DialogInput::ToggleDebug(active) => {
                self.show_debug = active;
            }
            DialogInput::SetName(index, name) => {
                if let Some(data) = self.rows_data.get_mut(index) {
                    data.name = name;
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.move_front = false;
        let DialogCommandOutput::Generated(seq, outcome) = msg;
        // A stale result: the dialog was reopened (new sequence number) or cancelled (hidden)
        // while this run was going. Drop it.
        if seq != self.build_seq || self.hidden {
            return;
        }
        self.done = true;
        self.results_data = result_data(&self.params, &self.coverages, &outcome);
        self.refresh_results();
        self.outcome = Some(outcome);
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}

impl Dialog {
    fn refresh_results(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.results,
            self.results_data.iter().cloned(),
            result_display::ListRowInput::UpdateData,
        );
    }
}
