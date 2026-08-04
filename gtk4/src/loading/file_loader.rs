use collomatique_storage::{Caveat, DecodeError, DeserializationError, IdKind, LoadError, RowKey};
use relm4::{Component, ComponentParts, ComponentSender};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug)]
pub enum FileLoadingInput {
    Load(PathBuf),
}

#[derive(Debug)]
pub enum FileLoadingOutput {
    Loaded(
        PathBuf,
        collomatique_state_colloscopes::Data,
        BTreeSet<Caveat>,
    ),
    Failed(PathBuf, String),
}

#[derive(Debug)]
pub enum FileLoadingCmdOutput {
    Loaded(
        PathBuf,
        collomatique_state_colloscopes::Data,
        BTreeSet<Caveat>,
    ),
    Failed(PathBuf, LoadError),
}

pub struct FileLoader;

impl Component for FileLoader {
    type Init = ();
    type Input = FileLoadingInput;
    type Output = FileLoadingOutput;
    type CommandOutput = FileLoadingCmdOutput;
    type Root = ();
    type Widgets = ();

    fn init_root() -> Self::Root {}

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        ComponentParts {
            model: FileLoader,
            widgets: (),
        }
    }

    fn update(&mut self, msg: FileLoadingInput, sender: ComponentSender<Self>, _root: &Self::Root) {
        let FileLoadingInput::Load(path) = msg;
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    out.send(
                        match collomatique_storage::load_data_from_file(&path).await {
                            Ok((data, caveats)) => {
                                FileLoadingCmdOutput::Loaded(path, data, caveats)
                            }
                            Err(e) => FileLoadingCmdOutput::Failed(path, e),
                        },
                    )
                    .unwrap();
                })
                .drop_on_shutdown()
        });
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            FileLoadingCmdOutput::Loaded(path, data, caveats) => {
                sender
                    .output(FileLoadingOutput::Loaded(path, data, caveats))
                    .unwrap();
            }
            FileLoadingCmdOutput::Failed(path, error) => {
                let error_msg = Self::generate_error_message(error);
                sender
                    .output(FileLoadingOutput::Failed(path, error_msg))
                    .unwrap();
            }
        }
    }
}

impl FileLoader {
    fn generate_error_message(error: LoadError) -> String {
        match error {
            LoadError::IO(io_error) => format!("Erreur lors de l'accès au fichier ({}).", io_error),
            LoadError::Deserialization(deserialization_error) => match deserialization_error {
                DeserializationError::InvalidJson(json_error) => format!(
                    "Le format de fichier semble incorrect ({}).\nVérifier s'il s'agit du bon fichier.",
                    json_error
                ),
                DeserializationError::Decode(decode_error) => {
                    Self::generate_decode_error_message(decode_error)
                }
                DeserializationError::RetiredSpec1Format => (
                    "Ce fichier utilise un format pré-alpha (spec 1) qui n'est plus pris en charge et ne peut plus être ouvert."
                )
                .into(),
                DeserializationError::UnsupportedSpecVersions { versions } => format!(
                    "Le fichier est mal formé et est probablement corrompu.\n(Combinaison de versions de spécification non prise en charge dans les entrées : {:?})",
                    versions
                ),
            },
        }
    }

    fn row_fr(row: &RowKey) -> String {
        match row {
            RowKey::Id(id) => format!("entrée id {}", id),
            RowKey::PeriodSubject {
                period_id,
                subject_id,
            } => format!("ligne (période {}, matière {})", period_id, subject_id),
        }
    }

    fn id_kind_fr(kind: &IdKind) -> &'static str {
        match kind {
            IdKind::Period => "une période inconnue",
            IdKind::Subject => "une matière inconnue",
            IdKind::Teacher => "un colleur inconnu",
            IdKind::Student => "un élève inconnu",
            IdKind::WeekPattern => "un motif de semaines inconnu",
            IdKind::Slot => "un créneau inconnu",
            IdKind::GroupList => "une liste de groupes inconnue",
        }
    }

    fn generate_decode_error_message(decode_error: DecodeError) -> String {
        match decode_error {
            DecodeError::EndOfTheUniverse => "Le fichier est probablement un fichier malicieux ou est corrompu.\n(Dernier ID utilisé supérieur à 2^63)".into(),
            DecodeError::DuplicatedID => "Le fichier est mal formé et est probablement corrompu.\n(ID en double)".into(),
            DecodeError::DuplicatedIdInBlock { block, id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(L'ID {} apparaît en double dans le bloc {})",
                id, block
            ),
            DecodeError::IdAboveCeiling { block, id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le bloc {} définit l'id {}, au-dessus du plafond autorisé (2^63 - 1))",
                block, id
            ),
            DecodeError::DuplicatedIdAcrossBlocks { first, second, id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(L'id {} est défini à la fois dans le bloc {} et dans le bloc {})",
                id, first, second
            ),
            DecodeError::MismatchedSpecRequirementInEntry(block) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Information de version erronée dans l'entrée du bloc {})",
                block
            ),
            DecodeError::ProbablyIllformedEntry => "Le fichier est mal formé et est probablement corrompu.\n(Entrée dans les spécifications mais non reconnue)".into(),
            DecodeError::UnknownNeededEntry(version) => format!(
                "Le fichier a été produit avec une version plus récente de Collomatique et ne peut être ouvert.\nUtiliser la version {} pour ouvrir ce fichier.",
                version
            ),
            DecodeError::UnknownFileType(version) => format!(
                "Type de fichier Collomatique inconnu.\nCe fichier a peut-être été produit avec une version plus récente ({}).",
                version
            ),
            DecodeError::UnknownFileContent(version) => format!(
                "Contenu de fichier Collomatique inconnu.\nCe fichier a peut-être été produit avec une version plus récente ({}).",
                version
            ),
            DecodeError::MalformedEntryContent => "Le fichier est mal formé et est probablement corrompu.\n(Le contenu d'une entrée n'est pas un objet avec exactement une clé)".into(),
            DecodeError::DuplicatedBlock(block) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le bloc {} apparaît plusieurs fois)",
                block
            ),
            DecodeError::IllformedBlock { block, detail } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le bloc {} est mal formé : {})",
                block, detail
            ),
            DecodeError::IncompatibilitySlotCrossesMidnight { incompat_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Un créneau de l'incompatibilité id {} dépasse minuit)",
                incompat_id
            ),
            DecodeError::UnknownSlotInColloscope(slot_id) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope référence un créneau inconnu, id {})",
                slot_id
            ),
            DecodeError::InvalidInterrogationCell { slot_id, week } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope place une interrogation sur une case inexistante : créneau {}, semaine {})",
                slot_id, week
            ),
            DecodeError::InterrogationGroupOutOfBounds { slot_id, week, group, group_count } => {
                if group_count == 0 {
                    format!(
                        "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope assigne le groupe {} sur la case (créneau {}, semaine {}), mais aucune liste de groupes n'est associée à cette matière sur cette période)",
                        group, slot_id, week
                    )
                } else {
                    format!(
                        "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope assigne le groupe {} sur la case (créneau {}, semaine {}), mais la liste de groupes associée n'a que {} groupes)",
                        group, slot_id, week, group_count
                    )
                }
            }
            DecodeError::InvalidColloscopeGroupList(group_list_id) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope remplit une liste de groupes invalide, id {})",
                group_list_id
            ),
            DecodeError::ColloscopeStudentExcluded { group_list_id, student_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope place l'élève id {} dans la liste de groupes id {}, qui exclut cet élève)",
                student_id, group_list_id
            ),
            DecodeError::ColloscopeStudentGroupOutOfBounds { group_list_id, student_id, group, group_count } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope place l'élève id {} de la liste de groupes id {} dans le groupe {}, mais la liste n'a que {} groupes)",
                student_id, group_list_id, group, group_count
            ),
            DecodeError::InconsistentGroupList(group_list_id) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Une liste de groupes est incohérente : nombre de groupes préremplis ou élève en double, id {})",
                group_list_id
            ),
            DecodeError::InconsistentPairingRule(rule_id) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Une règle d'appariement utilise la même matière des deux côtés, id {})",
                rule_id
            ),
            DecodeError::InconsistentSlotPairingRule(rule_id) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Un appariement de créneaux utilise le même créneau des deux côtés, id {})",
                rule_id
            ),
            DecodeError::UnknownPeriodInAssignments(period_id) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les affectations référencent une période inconnue, id {})",
                period_id
            ),
            DecodeError::UnknownSubjectInAssignments(subject_id) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les affectations référencent une matière inconnue, id {})",
                subject_id
            ),
            DecodeError::AssignmentOnExcludedPeriod { period_id, subject_id } => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les affectations ont une ligne pour la matière {} sur la période {}, dont elle est exclue)",
                subject_id, period_id
            ),
            DecodeError::AssignedStudentExcludedFromPeriod { period_id, subject_id, student_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(L'élève id {}, affecté dans la ligne (période {}, matière {}), est exclu de cette période)",
                student_id, period_id, subject_id
            ),
            DecodeError::TeacherSubjectWithoutInterrogations { teacher_id, subject_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colleur id {} référence la matière {}, qui n'a pas d'interrogations)",
                teacher_id, subject_id
            ),
            DecodeError::UnknownSubjectInSlots(subject_id) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les créneaux référencent une matière inconnue, id {})",
                subject_id
            ),
            DecodeError::SlotsForSubjectWithoutInterrogations(subject_id) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les créneaux ont une ligne pour la matière {}, qui n'a pas d'interrogations)",
                subject_id
            ),
            DecodeError::SlotTeacherDoesNotTeachSubject { slot_id, teacher_id, subject_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le créneau id {} nomme le colleur id {}, qui n'interroge pas dans la matière {})",
                slot_id, teacher_id, subject_id
            ),
            DecodeError::SlotOverflowsDay { slot_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le créneau id {}, avec la durée d'interrogation de sa matière, dépasse minuit)",
                slot_id
            ),
            DecodeError::DanglingReference { block, row, referenced, id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le bloc {}, {}, référence {} (id {}))",
                block, Self::row_fr(&row), Self::id_kind_fr(&referenced), id
            ),
            DecodeError::AssociationForSubjectWithoutInterrogations { period_id, subject_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(L'association de liste de groupes (période {}, matière {}) porte sur une matière sans interrogations)",
                period_id, subject_id
            ),
            DecodeError::AssociationOnExcludedPeriod { period_id, subject_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(L'association de liste de groupes (période {}, matière {}) porte sur une matière exclue de cette période)",
                period_id, subject_id
            ),
            DecodeError::PairingRuleForSubjectWithoutInterrogations { rule_id, subject_id } => format!(
                "Fichier mal formé et est probablement corrompu.\n(La règle d'appariement {} nomme la matière {}, qui n'a pas d'interrogations)",
                rule_id, subject_id
            ),
            DecodeError::SlotPairingAcrossSubjects { rule_id, antecedent_slot_id, consequent_slot_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(L'appariement de créneaux id {} associe les créneaux id {} et id {}, qui appartiennent à des matières différentes)",
                rule_id, antecedent_slot_id, consequent_slot_id
            ),
            DecodeError::BalancingForSubjectWithoutInterrogations { subject_id } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Les options d'équilibrage portent sur la matière id {}, qui n'a pas d'interrogations)",
                subject_id
            ),
            DecodeError::WrongWeekCountInWeekPattern { week_pattern_id, expected, found } => format!(
                "Fichier mal formé et est probablement corrompu.\n(Le motif de semaines {} décrit {} semaines alors que le calendrier en compte {})",
                week_pattern_id, found, expected
            ),
            DecodeError::LogicError(set) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les données sont logiquement impossibles : {})",
                set.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ; ")
            ),
            DecodeError::BrokenInvariants(set) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les données ne vérifient pas un invariant : {})",
                set.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ; ")
            ),
        }
    }
}
