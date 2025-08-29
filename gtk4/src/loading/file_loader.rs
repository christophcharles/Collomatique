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
            LoadError::IO(io_error) => format!(
                "Erreur lors de l'accès au fichier ({}).",
                io_error.to_string()
            ),
            LoadError::Deserialization(deserialization_error) => match deserialization_error {
                DeserializationError::InvalidJson(json_error) => format!(
                    "Le format de fichier semble incorrect ({}).\nVérifier s'il s'agit du bon fichier.",
                    json_error.to_string()
                ),
                DeserializationError::Decode(decode_error) => Self::generate_decode_error_message(decode_error),
            }
        }
    }

    fn generate_decode_error_message(decode_error: DecodeError) -> String {
        match decode_error {
            DecodeError::EndOfTheUniverse => "Le fichier est probablement un fichier malicieux ou est corrompu.\n(Dernier ID utilisé supérieur à 2^63)".into(),
            DecodeError::DuplicatedEntry(_) => "Le fichier est mal formé et est probablement corrompu.\n(Entrée en double)".into(),
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
        }
    }
}
