use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use collomatique::json::{json, Logic};
use collomatique::frontend::shell::CliCommand;
use collomatique::frontend::state::{AppState, Manager};

#[derive(Debug, Parser)]
#[clap(disable_help_flag = true)]
#[command(name = "")]
struct ShellLine {
    /// Command to run on the database
    #[command(subcommand)]
    command: ShellCommand,
}

#[derive(Debug, Subcommand)]
enum ShellCommand {
    #[command(flatten)]
    Global(CliCommand),
    #[command(flatten)]
    Extra(ShellExtraCommand),
}

#[derive(Debug, Subcommand)]
enum ShellExtraCommand {
    /// Undo last command
    Undo,
    /// Redo previously undone command
    Redo,
    PrintHistory,
    /// Save current data
    Save,
    /// Save current data
    SaveAs {
        /// Path to save to
        path: std::path::PathBuf,
    },
    /// Show current working file
    Cwf,
    /// Exit shell
    Exit {
        /// Force exit if modifications were not saved
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
}

fn open_collomatique_file(create: bool, path: &std::path::Path) -> Result<json::JsonStore> {
    if create {
        if path.try_exists()? {
            Err(anyhow!("Cannot create file - it already exists"))
        } else {
            Ok(json::JsonStore::new())
        }
    } else {
        Ok(json::JsonStore::from_json_file(path)?)
    }
}

struct ReedCompleter {}

impl reedline::Completer for ReedCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<reedline::Suggestion> {
        use clap::CommandFactory;
        use reedline::Span;
        use std::ffi::OsString;

        let mut cmd = ShellLine::command();
        let args = shlex::Shlex::new(line);
        let mut args = std::iter::once("".to_owned())
            .chain(args)
            .map(OsString::from)
            .collect::<Vec<_>>();
        if line.ends_with(' ') || line.is_empty() {
            args.push(OsString::new());
        }
        let arg_index = args.len() - 1;
        let span = Span::new(pos - args[arg_index].len(), pos);

        let current_dir = std::env::current_dir().ok();
        let current_dir_ref = current_dir.as_ref().map(|x| x.as_path());
        let Ok(candidates) =
            clap_complete::dynamic::complete(&mut cmd, args, arg_index, current_dir_ref)
        else {
            return vec![];
        };

        candidates
            .into_iter()
            .map(|c| reedline::Suggestion {
                value: c.0.to_string_lossy().into_owned(),
                description: c.1.map(|x| x.to_string()),
                style: None,
                extra: None,
                span,
                append_whitespace: true,
            })
            .collect()
    }
}

fn interactive_shell(
    app_state: &mut AppState<json::JsonStore>,
    mut current_path: std::path::PathBuf,
    mut initial_state_store: json::JsonStore,
) -> Result<()> {
    use nu_ansi_term::{Color, Style};
    use reedline::{
        DefaultHinter, Emacs, FileBackedHistory, IdeMenu, KeyCode, KeyModifiers, MenuBuilder,
        Reedline, ReedlineEvent, ReedlineMenu,
    };

    let completer = Box::new(ReedCompleter {});
    let completion_menu = Box::new(IdeMenu::default().with_name("completion_menu"));

    let mut keybindings = reedline::default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let mut rl = Reedline::create()
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_hinter(Box::new(
            DefaultHinter::default().with_style(Style::new().fg(Color::DarkGray).italic()),
        ))
        .with_edit_mode(Box::new(Emacs::new(keybindings)))
        .with_history(Box::new(FileBackedHistory::new(10000).unwrap()));

    loop {
        match respond(
            &mut rl,
            app_state,
            &mut current_path,
            &mut initial_state_store,
        ) {
            Ok(quit) => {
                if quit {
                    break;
                }
            }
            Err(err) => {
                use std::io::Write;
                writeln!(std::io::stderr(), "{err}")?;
                std::io::stderr().flush()?;
            }
        }
    }
    Ok(())
}

fn respond(
    rl: &mut reedline::Reedline,
    app_state: &mut AppState<json::JsonStore>,
    current_path: &mut std::path::PathBuf,
    initial_state_store: &mut json::JsonStore,
) -> Result<bool> {
    use reedline::{DefaultPrompt, DefaultPromptSegment, Signal};

    let mut prompt = DefaultPrompt::default();
    prompt.left_prompt = DefaultPromptSegment::Basic("collomatique".to_owned());
    prompt.right_prompt = DefaultPromptSegment::Empty;

    let line = match rl.read_line(&prompt) {
        Ok(Signal::Success(buffer)) => buffer,
        Ok(Signal::CtrlC | Signal::CtrlD) => return Ok(true),
        _ => return Err(anyhow!("Failed to read line. Input maybe invalid.")),
    };

    if line.trim().is_empty() {
        return Ok(false);
    }

    let shell_command = match shlex::split(&line) {
        Some(mut value) => {
            value.insert(0, String::from(""));
            match ShellLine::try_parse_from(value.iter().map(String::as_str)) {
                Ok(c) => c,
                Err(e) => return Err(e.into()),
            }
        }
        None => return Err(anyhow!("Invalid input")),
    };

    let output = match shell_command.command {
        ShellCommand::Global(command) => {
            collomatique::frontend::shell::execute_cli_command(command, app_state)?
        }
        ShellCommand::Extra(extra_command) => match extra_command {
            ShellExtraCommand::Undo => {
                app_state.undo()?;
                None
            }
            ShellExtraCommand::Redo => {
                app_state.redo()?;
                None
            }
            ShellExtraCommand::PrintHistory => {
                Some(format!("{:?}", app_state.get_aggregated_history()))
            }
            ShellExtraCommand::Save => {
                app_state
                    .get_logic()
                    .get_storage()
                    .to_json_file(&current_path)?;
                *initial_state_store = app_state.get_logic().get_storage().clone();
                None
            }
            ShellExtraCommand::SaveAs { path } => {
                app_state.get_logic().get_storage().to_json_file(&path)?;
                *initial_state_store = app_state.get_logic().get_storage().clone();
                *current_path = path;
                None
            }
            ShellExtraCommand::Cwf => Some(current_path.to_string_lossy().into_owned()),
            ShellExtraCommand::Exit { force } => {
                if (!force) && (*app_state.get_logic().get_storage() != *initial_state_store) {
                    return Err(anyhow!("Modifications were not saved"));
                }
                return Ok(true);
            }
        },
    };
    if let Some(msg) = output {
        println!("{}", msg);
    }

    Ok(false)
}

use super::CliCommandOrShell;

pub fn run_cli(create: bool, path: std::path::PathBuf, command: CliCommandOrShell) -> Result<()> {
    let initial_state_store = open_collomatique_file(create, path.as_path())?;
    let logic = Logic::new(initial_state_store.clone());

    collomatique::frontend::python::initialize();

    match command {
        CliCommandOrShell::Shell => {
            let mut app_state = AppState::new(logic);
            interactive_shell(&mut app_state, path, initial_state_store)?;
        }
        CliCommandOrShell::Global(command) => {
            let mut app_state = AppState::new(logic);
            let output =
                collomatique::frontend::shell::execute_cli_command(command, &mut app_state)?;
            if let Some(msg) = output {
                print!("{}", msg);
            }
            app_state.get_logic().get_storage().to_json_file(&path)?;
        }
    }

    Ok(())
}
