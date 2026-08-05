mod spec_row;

use std::collections::BTreeSet;

use adw::prelude::PreferencesGroupExt;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};

use collomatique_constraints_groups::{
    GenerationPlan, GenerationRequest, GroupListsModel, Var, build_generation_plan,
    build_incremental_epochs, build_model_with_log,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::{PeriodId, SubjectId};
use collomatique_strategies::{ConductorPayload, IncrementalPayload};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

/// Modal naming/build dialog: the second step of the generation chain. It turns the configured
/// request into a [`GenerationPlan`], shows one editable name per spec, reports the pairs nobody
/// is registered for, and builds the ILP model off the UI thread while streaming the build log
/// into a [`DebugView`]. "Valider" stays insensitive until the model is built.
pub struct Dialog {
    hidden: bool,
    /// Toggles the content between the naming rows and the build log.
    show_debug: bool,
    /// Flips the header indicator from spinner to "ok". Distinct from `model.is_some()`: with
    /// zero specs there is nothing to build, so the indicator shows "done" while "Valider" stays
    /// insensitive.
    built: bool,
    /// Discards build results from a superseded `Show` (or from after a cancel).
    build_seq: u64,
    /// The plan the rows and the build were made from; echoed back on `Accepted`.
    plan: Option<GenerationPlan>,
    /// `Some` once the off-thread build has finished; consumed by `Accept`.
    model: Option<GroupListsModel>,
    /// The skipped-pairs warning; empty when nothing was skipped.
    skipped_text: String,
    /// Mirror of the rows' current titles and names, in plan/spec order.
    rows_data: Vec<spec_row::Data>,
    rows: FactoryVecDeque<spec_row::SpecRow>,
    debug_view: Controller<DebugView>,
}

#[derive(Debug)]
pub enum DialogInput {
    /// Open the dialog for this request, against the parameters the config dialog echoed back.
    Show(GenerationRequest, Parameters),
    Cancel,
    Accept,
    /// One build-log line, streamed from the off-thread build.
    Echo(String),
    ToggleDebug(bool),
    /// (spec index, new name), forwarded from a row.
    SetName(usize, String),
}

#[derive(Debug)]
pub enum DialogOutput {
    /// The plan, one name per spec (in plan order), the built model and the conductor payload
    /// the solver dialog needs.
    Accepted(
        GenerationPlan,
        Vec<String>,
        GroupListsModel,
        ConductorPayload<Var>,
    ),
    Cancelled,
}

#[derive(Debug)]
pub enum DialogCommandOutput {
    /// (the `build_seq` this build was spawned with, the built model)
    Built(u64, GroupListsModel),
}

/// "a", "a et b", "a, b et c" — the French enumeration join.
fn french_join(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [single] => single.clone(),
        [head @ .., last] => format!("{} et {}", head.join(", "), last),
    }
}

/// What one spec covers: "Maths (période 1)", "Maths et Physique (périodes 1 et 2)". Also used
/// as the default list name — distinct specs cover disjoint pair sets, so these are unique.
/// Subjects come out in document display order, periods as 1-based positions in display order.
fn coverage_label(params: &Parameters, covered: &BTreeSet<(PeriodId, SubjectId)>) -> String {
    let subject_ids: BTreeSet<SubjectId> = covered.iter().map(|&(_, subject)| subject).collect();
    let period_ids: BTreeSet<PeriodId> = covered.iter().map(|&(period, _)| period).collect();

    let subjects: Vec<String> = params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(id, _subject)| subject_ids.contains(id))
        .map(|(_id, subject)| subject.parameters.name.clone())
        .collect();

    // Periods have no name: the 1-based position is what the whole UI shows.
    let periods: Vec<String> = params
        .periods
        .period_ids()
        .enumerate()
        .filter(|(_pos, id)| period_ids.contains(id))
        .map(|(pos, _id)| (pos + 1).to_string())
        .collect();

    let period_part = if periods.len() == 1 {
        format!("période {}", periods[0])
    } else {
        format!("périodes {}", french_join(&periods))
    };

    format!("{} ({})", french_join(&subjects), period_part)
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

#[relm4::component(pub)]
impl Component for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;
    type CommandOutput = DialogCommandOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Nommer les nouvelles listes de groupes"),
            set_default_size: (700, 550),
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
                        // Nothing to validate until the model is built — and with no spec at
                        // all, nothing ever gets built.
                        #[watch]
                        set_sensitive: model.model.is_some(),
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
                                set_visible: !model.built,
                            },
                            gtk::Image::from_icon_name("emblem-ok-symbolic") {
                                #[watch]
                                set_visible: model.built,
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
                        #[local_ref]
                        rows_box -> adw::PreferencesGroup {
                            set_hexpand: true,
                            set_margin_all: 5,
                            set_title: "Noms des listes générées",
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

        let debug_view = DebugView::builder().launch(()).detach();

        let model = Dialog {
            hidden: true,
            show_debug: false,
            built: false,
            build_seq: 0,
            plan: None,
            model: None,
            skipped_text: String::new(),
            rows_data: Vec::new(),
            rows,
            debug_view,
        };

        let rows_box = model.rows.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DialogInput::Show(request, params) => {
                self.hidden = false;
                self.show_debug = false;
                self.built = false;
                self.model = None;
                // Any build still running from a previous opening is now stale.
                self.build_seq += 1;
                self.debug_view.emit(DebugViewInput::Clear);

                // The config dialog only offers valid choices, and both dialogs are modal, so
                // the document cannot have changed in between: a plan error here is a caller
                // bug (that is exactly what `GenerationPlanError` documents).
                let plan = build_generation_plan(&params, &request)
                    .expect("the config dialog assembled the request against these parameters");

                self.skipped_text = skipped_label(&params, &plan.skipped);
                self.rows_data = plan
                    .specs
                    .iter()
                    .map(|(_spec, covered)| {
                        let coverage = coverage_label(&params, covered);
                        spec_row::Data {
                            title: format!("Liste pour {}", coverage),
                            name: coverage,
                        }
                    })
                    .collect();
                crate::tools::factories::update_vec_deque(
                    &mut self.rows,
                    self.rows_data.iter().cloned(),
                    spec_row::SpecRowInput::UpdateData,
                );

                if plan.specs.is_empty() {
                    // Every selected pair was skipped: nothing to name and nothing to build.
                    // The dialog still opens — it is where the user learns why — but "Valider"
                    // stays insensitive, since no model will ever exist.
                    self.built = true;
                    self.plan = Some(plan);
                } else {
                    // Building the model is heavy work (phase B grows real constraints here).
                    // Run it off the UI thread; each log line is emitted back as `Echo` and
                    // streams live into the DebugView while the build runs.
                    let seq = self.build_seq;
                    let build_plan = plan.clone();
                    self.plan = Some(plan);
                    let input = sender.input_sender().clone();
                    sender.spawn_oneshot_command(move || {
                        let mut log = move |line: &str| {
                            input.emit(DialogInput::Echo(format!("{}\n", line)));
                        };
                        let model = build_model_with_log(&build_plan, &mut log);
                        DialogCommandOutput::Built(seq, model)
                    });
                }
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                // "Valider" is insensitive until the model exists, so a missing model here can
                // only come from a race; ignore it.
                let Some(model) = self.model.take() else {
                    return;
                };
                let plan = self
                    .plan
                    .take()
                    .expect("a built model implies a stored plan");
                let names: Vec<String> = self.rows_data.iter().map(|d| d.name.clone()).collect();

                // The inclusion-based epochs of §2.6: inclusion-minimal lists solve
                // first, and each larger list aligns with the groups already fixed
                // inside it through the pair objective. Cheap (quadratic in the spec
                // count), so it runs right here rather than alongside the off-thread
                // model build.
                let payload = ConductorPayload {
                    incremental: IncrementalPayload {
                        epochs: build_incremental_epochs(&plan),
                    },
                };

                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(plan, names, model, payload))
                    .unwrap();
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
        let DialogCommandOutput::Built(seq, model) = msg;
        // A stale result: the dialog was reopened (new sequence number) or cancelled (hidden)
        // while this build was running. Drop it.
        if seq != self.build_seq || self.hidden {
            return;
        }
        self.built = true;
        self.model = Some(model);
    }
}
