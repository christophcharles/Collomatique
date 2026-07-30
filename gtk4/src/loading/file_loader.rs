use collomatique_storage::{Caveat, DecodeError, DeserializationError, LoadError};
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

    fn generate_decode_error_message(decode_error: DecodeError) -> String {
        match decode_error {
            DecodeError::EndOfTheUniverse => "Le fichier est probablement un fichier malicieux ou est corrompu.\n(Dernier ID utilisé supérieur à 2^63)".into(),
            DecodeError::DuplicatedID => "Le fichier est mal formé et est probablement corrompu.\n(ID en double)".into(),
            DecodeError::MismatchedSpecRequirementInEntry => "Le fichier est mal formé et est probablement corrompu.\n(Information de version erronée dans une entrée)".into(),
            DecodeError::ProbablyIllformedEntry => "Le fichier est mal formé et est probablement corrompu.\n(Entrée dans les spécifications mais non reconnue)".into(),
            DecodeError::UnknownNeededEntry(version) => format!(
                "Le fichier a été produit avec une version plus récente de Collomatique et ne peut être ouvert.\nUtiliser la version {} pour ouvrir ce fichier.",
                version
            ),
            DecodeError::UnknownFileType(version) => format!(
                "Type de fichier Collomatique inconnu.\nCe fichier a peut-être été produit avec une version plus récente ({}).",
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
            DecodeError::SlotCrossesMidnight => "Le fichier est mal formé et est probablement corrompu.\n(Un créneau d'incompatibilité dépasse minuit)".into(),
            DecodeError::UnknownSlotInColloscope(slot_id) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope référence un créneau inconnu, id {})",
                slot_id
            ),
            DecodeError::InvalidInterrogationCell { slot_id, week } => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope place une interrogation sur une case inexistante : créneau {}, semaine {})",
                slot_id, week
            ),
            DecodeError::InvalidColloscopeGroupList(group_list_id) => format!(
                "Le fichier est mal formé et est probablement corrompu.\n(Le colloscope remplit une liste de groupes invalide, id {})",
                group_list_id
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
            DecodeError::UnknownSubjectInSlots(subject_id) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les créneaux référencent une matière inconnue, id {})",
                subject_id
            ),
            DecodeError::SlotsForSubjectWithoutInterrogations(subject_id) => format!(
                "Fichier mal formé et est probablement corrompu.\n(Les créneaux ont une ligne pour la matière {}, qui n'a pas d'interrogations)",
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
