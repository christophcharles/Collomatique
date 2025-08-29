use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use collomatique::backend::{sqlite, Logic};
use collomatique::frontend::shell::CliCommand;
use collomatique::frontend::state::AppState;

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
    /// Exit shell
    Exit,
}

async fn connect_db(create: bool, path: &std::path::Path) -> Result<sqlite::Store> {
    if create {
        Ok(sqlite::Store::new_db(path).await?)
    } else {
        Ok(sqlite::Store::open_db(path).await?)
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

async fn interactive_shell(app_state: &mut AppState<sqlite::Store>) -> Result<()> {
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
        match respond(&mut rl, app_state).await {
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

async fn respond(
    rl: &mut reedline::Reedline,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<bool> {
    use collomatique::frontend::state::Manager;
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
            collomatique::frontend::shell::execute_cli_command(command, app_state).await?
        }
        ShellCommand::Extra(extra_command) => match extra_command {
            ShellExtraCommand::Undo => {
                app_state.undo().await?;
                None
            }
            ShellExtraCommand::Redo => {
                app_state.redo().await?;
                None
            }
            ShellExtraCommand::PrintHistory => {
                Some(format!("{:?}", app_state.get_aggregated_history()))
            }
            ShellExtraCommand::Exit => return Ok(true),
        },
    };
    if let Some(msg) = output {
        println!("{}", msg);
    }

    Ok(false)
}

use super::CliCommandOrShell;

async fn async_cli(create: bool, db: std::path::PathBuf, command: CliCommandOrShell) -> Result<()> {
    let logic = Logic::new(connect_db(create, db.as_path()).await?);
    let mut app_state = AppState::new(logic);

    collomatique::frontend::python::initialize();

    match command {
        CliCommandOrShell::Shell => {
            interactive_shell(&mut app_state).await?;
        }
        CliCommandOrShell::Global(command) => {
            let output =
                collomatique::frontend::shell::execute_cli_command(command, &mut app_state).await?;
            if let Some(msg) = output {
                print!("{}", msg);
            }
        }
    }

    Ok(())
}

pub fn run_cli(create: bool, db: std::path::PathBuf, command: CliCommandOrShell) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_cli(create, db, command))
}
