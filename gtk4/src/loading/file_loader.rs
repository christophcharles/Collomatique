use relm4::{ComponentSender, Worker};
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

pub struct FileLoader;

impl Worker for FileLoader {
    type Init = ();
    type Input = FileLoadingInput;
    type Output = FileLoadingOutput;

    fn init(_init: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, msg: FileLoadingInput, sender: ComponentSender<Self>) {
        // Simulating heavy CPU-bound task
        use std::time::Duration;
        std::thread::sleep(Duration::from_secs(3));

        let FileLoadingInput::Load(path) = msg;
        let data = collomatique_state_colloscopes::Data::new();
        /*sender
        .output(FileLoadingOutput::Loaded(path, data))
        .unwrap();*/
        sender
            .output(FileLoadingOutput::Failed(path, "Test error".into()))
            .unwrap();
    }
}
