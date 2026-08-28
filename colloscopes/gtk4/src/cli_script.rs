//! The command line's script mode, with a document
//!
//! `--python`/`--python-file` run a script instead of the application. The
//! positional `[FILE]` (or `--new`) names the document the script is hosted
//! with, the way the GUI's script editor hosts the open document: the script
//! reads it through `current_document()` and hands one back through
//! `send_to_host` — or `doc.save()` with no path, which is the same thing.
//! `--out` is where the held document goes when the script ends well; it may
//! name the file that was opened, and then overwrites it — the user typed the
//! path, the same consent rule as naming a path from python.
//!
//! `RpcHost` in `colloscopes/rpc-engine-colloscopes/src/lib.rs` is this same
//! shape over a pipe; here both ends are this process, so `send` just replaces
//! what it holds — and remembers that it did, which is what the dropped-work
//! warning is answered from. Without `[FILE]` and without `--new` there is no
//! host at all, as before: `current_document()` is None and the script works
//! on files it names itself.

use anyhow::Context as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use collomatique_python_runner::{EngineExe, Host, SendError, TakenDocument};
use collomatique_state_colloscopes::Data;

/// This process, as the hosted script's application
struct CliHost {
    state: Mutex<HostState>,
}

struct HostState {
    /// The document this process holds: what was loaded (or an empty one),
    /// then whatever the script sent last — sending twice is allowed, and the
    /// last one wins.
    data: Data,
    /// Whether `send` was ever called. That is all "modified" means here: a
    /// script sends explicitly, so there is no diffing to do.
    modified: bool,
}

impl Host for CliHost {
    fn live(&self) -> bool {
        false
    }

    fn data(&self) -> Result<TakenDocument, String> {
        Ok(TakenDocument {
            data: self.state.lock().unwrap().data.clone(),
            token: None,
        })
    }

    fn send(&self, data: &Data, _token: Option<u64>) -> Result<Option<u64>, SendError> {
        let mut state = self.state.lock().unwrap();
        state.data = data.clone();
        state.modified = true;
        Ok(None)
    }
}

/// Runs one command-line script: load, run, then save — or warn
///
/// On success with `--out`, the held document is written even when the script
/// never sent: the output is then the unmodified input, which is faithful. On
/// failure nothing is written, and either way, sent work that no file will
/// keep is warned about on stderr rather than dropped quietly.
pub fn run(
    code: String,
    new: bool,
    file: Option<PathBuf>,
    out: Option<PathBuf>,
    engine: Option<EngineExe>,
) -> anyhow::Result<()> {
    // The GUI reads `--new FILE` as "a new document destined for FILE"; a
    // script's document has `--out` for its destination, so here the pair
    // would be two different documents to start from.
    if new && let Some(path) = &file {
        anyhow::bail!(
            "--new opens an empty document and {} opens that file; \
             a script is hosted with one document, so pick one",
            path.display()
        );
    }

    let data = match &file {
        Some(path) => Some(load_document(path)?),
        None if new => Some(Data::new()),
        None => None,
    };

    if out.is_some() && data.is_none() {
        anyhow::bail!(
            "--out saves the document the script was hosted with, and there is \
             none: pass a file to open, or --new for an empty one"
        );
    }

    let host = data.map(|data| {
        Arc::new(CliHost {
            state: Mutex::new(HostState {
                data,
                modified: false,
            }),
        })
    });

    collomatique_python_runner::initialize();
    let result = collomatique_python_runner::run_python_script(
        code,
        host.clone().map(|host| host as Arc<dyn Host>),
        engine,
    );

    if let Some(host) = &host {
        let state = host.state.lock().unwrap();
        match (&result, &out) {
            (Ok(()), Some(path)) => save_document(&state.data, path)?,
            // Nothing will keep what was sent, and the two reasons read
            // differently to whoever ran this: with a destination given, the
            // failure is what stopped the write, and naming `--out` would be
            // advice for a mistake they did not make.
            (Err(_), Some(_)) if state.modified => {
                eprintln!(
                    "{}",
                    collomatique_ui_text::script::interrupted_modifications_text()
                );
            }
            (_, None) if state.modified => {
                eprintln!(
                    "{}",
                    collomatique_ui_text::script::lost_modifications_text()
                );
            }
            _ => {}
        }
    }

    result
}

/// Reads a colloscope file the way the application does
///
/// `deserialize_data` and then the invariant gate, the pair the gtk4 file
/// loader runs — synchronously here, like the python module's `load`. The
/// caveats go to stderr, one sentence per line: this is the process talking to
/// the human who invoked it, the run itself continues, and the GUI continues
/// after showing them too.
fn load_document(path: &Path) -> anyhow::Result<Data> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;

    let (inner_data, caveats) = collomatique_storage::deserialize_data(&content)
        .with_context(|| format!("cannot decode {}", path.display()))?;

    if !caveats.is_empty() {
        eprintln!(
            "{}",
            collomatique_ui_text::script::load_caveats_intro_text(path)
        );
        for caveat in &caveats {
            eprintln!("- {}", collomatique_ui_text::caveats::caveat_text(caveat));
        }
    }

    Data::from_inner_data(inner_data)
        .with_context(|| format!("{} does not hold a valid document", path.display()))
}

/// Writes the held document where `--out` said
fn save_document(data: &Data, path: &Path) -> anyhow::Result<()> {
    let content = collomatique_storage::serialize_data(data.get_inner_data())
        .with_context(|| format!("cannot encode the document for {}", path.display()))?;

    std::fs::write(path, content).with_context(|| format!("cannot write {}", path.display()))
}
