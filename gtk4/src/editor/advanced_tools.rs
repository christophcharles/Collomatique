use gtk::prelude::{BoxExt, ButtonExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};
use relm4::{adw, gtk};

use collomatique_constraints_colloscopes::IlpInnerProblem;

/// Counts shown in the "Statistiques du document" section.
///
/// The counting happens here, on the editor side, rather than in the panel:
/// only the counters cross the channel, never a clone of the document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub periods: usize,
    pub weeks: usize,
    pub interrogation_weeks: usize,
    pub subjects: usize,
    pub subjects_with_interrogations: usize,
    pub teachers: usize,
    pub students: usize,
    pub week_patterns: usize,
    pub slots: usize,
    pub incompats: usize,
    pub group_lists: usize,
    pub pairing_rules: usize,
    pub slot_pairing_rules: usize,
    pub assignments: usize,
    pub placed_interrogations: usize,
    pub possible_interrogations: usize,
    pub entity_count: usize,
    pub max_id: Option<u64>,
}

impl Stats {
    pub fn from_inner_data(data: &collomatique_state_colloscopes::InnerData) -> Self {
        let params = &data.params;

        let interrogation_weeks = params
            .walk_weeks()
            .filter(|(_, _, w)| w.interrogations)
            .count();

        let subjects_with_interrogations = params
            .subjects
            .ordered_subject_list
            .values()
            .filter(|subject| subject.parameters.interrogation_parameters.is_some())
            .count();

        let assignments = params
            .assignments
            .iter()
            .map(|(_period, _subject, students)| students.len())
            .sum();

        let placed_interrogations = data
            .colloscope
            .iter()
            .filter(|(_key, groups)| !groups.is_empty())
            .count();

        // The possibility oracle is per (slot, week) pair, so this is a full
        // cross-product walk. That is cheap next to what the surrounding
        // interface update already does (it clones every table of the document).
        let week_ids: Vec<_> = params.week_ids().collect();
        let mut possible_interrogations = 0;
        for (slot_id, _slot) in params.slots.all_slots() {
            for week_id in &week_ids {
                if params.is_interrogation_possible(*slot_id, *week_id) {
                    possible_interrogations += 1;
                }
            }
        }

        // `all_ids` only walks the parameter tables, not the colloscope. On a
        // valid document every colloscope id also names a live entity, so the
        // count and the maximum are the same either way.
        let mut entity_count = 0;
        let mut max_id: Option<u64> = None;
        for id in params.all_ids() {
            entity_count += 1;
            let raw = id.inner();
            max_id = Some(match max_id {
                Some(current) => current.max(raw),
                None => raw,
            });
        }

        Stats {
            periods: params.periods.period_count(),
            weeks: params.count_weeks(),
            interrogation_weeks,
            subjects: params.subjects.ordered_subject_list.len(),
            subjects_with_interrogations,
            teachers: params.teachers.teacher_map.len(),
            students: params.students.student_map.len(),
            week_patterns: params.week_patterns.week_pattern_map.len(),
            slots: params.slots.all_slots().count(),
            incompats: params.incompats.incompat_map.len(),
            group_lists: params.group_lists.group_list_map.len(),
            pairing_rules: params.pairings.pairing_rule_map.len(),
            slot_pairing_rules: params.slot_pairings.slot_pairing_rule_map.len(),
            assignments,
            placed_interrogations,
            possible_interrogations,
            entity_count,
            max_id,
        }
    }
}

/// The size of the current ILP problem, when there is one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IlpProblemInfo {
    pub base_variables: usize,
    pub extra_variables: usize,
    pub helper_variables: usize,
    pub user_constraints: usize,
    pub defining_constraints: usize,
}

impl IlpProblemInfo {
    /// The counterpart of [Stats::from_inner_data]: the panel only ever
    /// receives counters, never the problem itself.
    pub fn from_problem(problem: &IlpInnerProblem) -> Self {
        use collomatique_constraints_colloscopes::{ConstraintSource, InternalVar};

        let mut info = IlpProblemInfo::default();

        for var in problem.get_variables().keys() {
            match var {
                InternalVar::Base(_) => info.base_variables += 1,
                InternalVar::Extra(_) => info.extra_variables += 1,
                InternalVar::Helper { .. } => info.helper_variables += 1,
            }
        }

        for (_constraint, source) in problem.get_constraints() {
            match source {
                ConstraintSource::User(_) => info.user_constraints += 1,
                ConstraintSource::DefiningExtra { .. } => info.defining_constraints += 1,
            }
        }

        info
    }

    fn total_variables(&self) -> usize {
        self.base_variables + self.extra_variables + self.helper_variables
    }

    fn total_constraints(&self) -> usize {
        self.user_constraints + self.defining_constraints
    }
}

pub struct AdvancedTools {
    stats: Stats,
    ilp_info: Option<IlpProblemInfo>,
}

#[derive(Debug)]
pub enum AdvancedToolsInput {
    Update(Stats),
    UpdateIlpProblemInfo(Option<IlpProblemInfo>),

    RunPythonScriptClicked,
    ExportMpsClicked,
    CompactIdsClicked,
}

#[derive(Debug)]
pub enum AdvancedToolsOutput {
    RunPythonScriptClicked,
    ExportMpsClicked,
    CompactIdsClicked,
}

impl AdvancedTools {
    /// Every statistic line, as one markup block.
    ///
    /// A single label rather than one widget per line: the set is fixed and
    /// purely textual, so there is nothing to gain from separate widgets.
    fn generate_stats_text(&self) -> String {
        let stats = &self.stats;

        let mut lines = vec![
            format!("<b>Périodes :</b> {}", stats.periods),
            format!(
                "<b>Semaines :</b> {} (dont {} avec colles)",
                stats.weeks, stats.interrogation_weeks
            ),
            format!(
                "<b>Matières :</b> {} (dont {} avec colles)",
                stats.subjects, stats.subjects_with_interrogations
            ),
            format!("<b>Colleurs :</b> {}", stats.teachers),
            format!("<b>Élèves :</b> {}", stats.students),
            format!("<b>Modèles de périodicité :</b> {}", stats.week_patterns),
            format!("<b>Créneaux de colles :</b> {}", stats.slots),
            format!("<b>Incompatibilités horaires :</b> {}", stats.incompats),
            format!("<b>Listes de groupes :</b> {}", stats.group_lists),
            format!("<b>Appariements de matières :</b> {}", stats.pairing_rules),
            format!(
                "<b>Appariements de créneaux :</b> {}",
                stats.slot_pairing_rules
            ),
            format!(
                "<b>Inscriptions dans les matières :</b> {}",
                stats.assignments
            ),
            format!(
                "<b>Colles placées :</b> {} (sur {} possibles)",
                stats.placed_interrogations, stats.possible_interrogations
            ),
            format!(
                "<b>Identifiants :</b> {} entités, plus grand identifiant : {}",
                stats.entity_count,
                match stats.max_id {
                    Some(id) => id.to_string(),
                    None => "aucun".to_string(),
                }
            ),
        ];

        match &self.ilp_info {
            Some(info) => {
                lines.push(format!(
                    "<b>Variables ILP :</b> {} ({} de base, {} intermédiaires, {} auxiliaires)",
                    info.total_variables(),
                    info.base_variables,
                    info.extra_variables,
                    info.helper_variables,
                ));
                lines.push(format!(
                    "<b>Contraintes ILP :</b> {} ({} du modèle, {} de définition)",
                    info.total_constraints(),
                    info.user_constraints,
                    info.defining_constraints,
                ));
            }
            None => {
                lines.push("<b>Variables ILP :</b> non calculées".to_string());
                lines.push("<b>Contraintes ILP :</b> non calculées".to_string());
            }
        }

        lines.join("\n")
    }
}

#[relm4::component(pub)]
impl Component for AdvancedTools {
    type Input = AdvancedToolsInput;
    type Output = AdvancedToolsOutput;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_vexpand: true,
            gtk::Box {
                set_margin_top: 30,
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_spacing: 15,
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 10,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "<b><i><big>Statistiques du document</big></i></b>",
                        set_use_markup: true,
                        set_margin_all: 5,
                        set_margin_bottom: 10,
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_margin_start: 10,
                        set_margin_end: 10,
                        set_use_markup: true,
                        #[watch]
                        set_label: &model.generate_stats_text(),
                    },
                },
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 10,
                    set_margin_top: 30,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },
                    gtk::Label {
                        set_label: "<b><i><big>Outils</big></i></b>",
                        set_use_markup: true,
                        set_margin_all: 5,
                        set_margin_bottom: 10,
                    },
                    gtk::Button {
                        add_css_class: "frame",
                        add_css_class: "warning",
                        set_hexpand: true,
                        set_margin_start: 10,
                        set_margin_end: 10,
                        set_size_request: (-1, 40),
                        adw::ButtonContent {
                            set_icon_name: "text-x-script",
                            set_label: "Exécuter un script Python",
                        },
                        connect_clicked => AdvancedToolsInput::RunPythonScriptClicked,
                    },
                    gtk::Button {
                        add_css_class: "frame",
                        add_css_class: "warning",
                        set_hexpand: true,
                        set_margin_start: 10,
                        set_margin_end: 10,
                        set_size_request: (-1, 40),
                        #[watch]
                        set_sensitive: model.ilp_info.is_some(),
                        adw::ButtonContent {
                            set_icon_name: "document-export-symbolic",
                            set_label: "Exporter le problème ILP (MPS)",
                        },
                        connect_clicked => AdvancedToolsInput::ExportMpsClicked,
                    },
                    gtk::Button {
                        add_css_class: "frame",
                        add_css_class: "warning",
                        set_hexpand: true,
                        set_margin_start: 10,
                        set_margin_end: 10,
                        set_size_request: (-1, 40),
                        adw::ButtonContent {
                            set_icon_name: "application-x-compress",
                            set_label: "Compacter les identifiants",
                        },
                        connect_clicked => AdvancedToolsInput::CompactIdsClicked,
                    },
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AdvancedTools {
            stats: Stats::default(),
            ilp_info: None,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            AdvancedToolsInput::Update(stats) => {
                self.stats = stats;
            }
            AdvancedToolsInput::UpdateIlpProblemInfo(info) => {
                self.ilp_info = info;
            }
            AdvancedToolsInput::RunPythonScriptClicked => {
                sender
                    .output(AdvancedToolsOutput::RunPythonScriptClicked)
                    .unwrap();
            }
            AdvancedToolsInput::ExportMpsClicked => {
                sender
                    .output(AdvancedToolsOutput::ExportMpsClicked)
                    .unwrap();
            }
            AdvancedToolsInput::CompactIdsClicked => {
                sender
                    .output(AdvancedToolsOutput::CompactIdsClicked)
                    .unwrap();
            }
        }
    }
}
