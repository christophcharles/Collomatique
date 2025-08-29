use relm4::{Component, ComponentParts, ComponentSender};
use std::path::PathBuf;

#[derive(Debug)]
pub enum FileLoadingInput {
    Load(PathBuf),
}

#[derive(Debug)]
pub enum FileLoadingOutput {
    Loaded(PathBuf, collomatique_state_colloscopes::Data),
    Failed(PathBuf, String),
}

#[derive(Debug)]
pub enum FileLoadingCmdOutput {
    Loaded(PathBuf, collomatique_state_colloscopes::Data),
    Failed(PathBuf, String),
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
                            Ok((data, _caveats)) => FileLoadingCmdOutput::Loaded(path, data),
                            Err(e) => FileLoadingCmdOutput::Failed(path, e.to_string()),
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
            FileLoadingCmdOutput::Loaded(path, data) => {
                sender
                    .output(FileLoadingOutput::Loaded(path, data))
                    .unwrap();
            }
            FileLoadingCmdOutput::Failed(path, error) => {
                sender
                    .output(FileLoadingOutput::Failed(path, error))
                    .unwrap();
            }
        }
    }
}
