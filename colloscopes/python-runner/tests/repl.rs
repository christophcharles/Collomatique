//! The interactive console, driven by a scripted keyboard
//!
//! The lines come from a fake `ReplIo` and the document from a fake `Host`, so
//! the whole session runs in this process: what is under test is that a line
//! typed at the console reaches the interpreter, that the session keeps its
//! globals between lines, and that the token the application gave travels with
//! the document and is updated by each send.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Once};

use collomatique_python_runner::{Host, ReplIo, SendError, TakenDocument};
use collomatique_state_colloscopes::Data;

static INIT: Once = Once::new();

/// One session at a time, whatever cargo does with its threads
///
/// The host a session sees is process-global, so two of them must not overlap.
static ONE_SESSION_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Registers the module and starts the interpreter, at most once per process
fn interpreter() {
    INIT.call_once(collomatique_python_runner::initialize);
}

/// A keyboard that was typed at in advance
///
/// Running out of lines is end of input, which is how the console ends.
struct FakeReplIo {
    state: Mutex<FakeReplState>,
}

struct FakeReplState {
    lines: VecDeque<String>,
    prompts: Vec<String>,
}

impl FakeReplIo {
    fn new(lines: impl IntoIterator<Item = &'static str>) -> FakeReplIo {
        FakeReplIo {
            state: Mutex::new(FakeReplState {
                lines: lines.into_iter().map(String::from).collect(),
                prompts: Vec::new(),
            }),
        }
    }

    fn prompts(&self) -> Vec<String> {
        self.state
            .lock()
            .expect("no reader panicked")
            .prompts
            .clone()
    }
}

impl ReplIo for FakeReplIo {
    fn read_line(&self, prompt: &str) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();
        state.prompts.push(prompt.to_owned());
        state
            .lines
            .pop_front()
            .ok_or_else(|| String::from("no more lines"))
    }
}

/// The application, as the console talks to it
///
/// It names the document it hands over, and every document it accepts, with a
/// token of its own, and records the token each send came with.
struct FakeHost {
    state: Mutex<FakeHostState>,
}

struct FakeHostState {
    token: u64,
    sent: Vec<Option<u64>>,
}

impl FakeHost {
    fn sent(&self) -> Vec<Option<u64>> {
        self.state.lock().expect("no sender panicked").sent.clone()
    }
}

impl Host for FakeHost {
    fn live(&self) -> bool {
        true
    }

    fn data(&self) -> Result<TakenDocument, String> {
        Ok(TakenDocument {
            data: Data::new(),
            token: Some(self.state.lock().unwrap().token),
        })
    }

    fn send(&self, _data: &Data, token: Option<u64>) -> Result<Option<u64>, SendError> {
        let mut state = self.state.lock().unwrap();
        state.sent.push(token);
        state.token += 1;
        Ok(Some(state.token))
    }
}

#[test]
fn the_console_runs_a_session_of_typed_lines() {
    let _one_at_a_time = ONE_SESSION_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    interpreter();

    let io = Arc::new(FakeReplIo::new([
        "doc = clm.current_document()",
        "clm.send_to_host(doc)",
        "clm.send_to_host(doc)",
        "if True:",
        "    x = 1",
        "",
        "clm.send_to_host(doc) if x == 1 else None",
    ]));
    let host = Arc::new(FakeHost {
        state: Mutex::new(FakeHostState {
            token: 42,
            sent: Vec::new(),
        }),
    });

    collomatique_python_runner::run_python_repl(Some(host.clone()), None, io.clone())
        .expect("the session ends when the lines run out");

    // The document arrived named `Some(42)`, and each send renamed it: a
    // console that takes, sends, edits and sends again never carries a stale
    // token.
    assert_eq!(host.sent(), vec![Some(42), Some(43), Some(44)]);

    // `doc` and `x` outlive the statement that defined them, and a compound
    // statement is asked for its continuation lines.
    let prompts = io.prompts();
    assert!(prompts.iter().any(|prompt| prompt == ">>> "));
    assert!(prompts.iter().any(|prompt| prompt == "... "));
}

/// The session asks for plain text rather than for a terminal
///
/// The worker's output is a pty, so every isatty() check inside it says
/// terminal, and `help()` would hand its text to a pager that writes escape
/// sequences and then waits for a keypress nobody can send. What the session
/// prints goes nowhere this test can read, so the session answers by sending a
/// document: one send means it found what it needs, none means it did not.
#[test]
fn the_console_asks_for_plain_text_output() {
    let _one_at_a_time = ONE_SESSION_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    interpreter();

    let io = Arc::new(FakeReplIo::new([
        "import os",
        "plain = os.environ[\"TERM\"] == \"dumb\" and \"PAGER\" not in os.environ",
        "clm.send_to_host(clm.current_document()) if plain else None",
    ]));
    let host = Arc::new(FakeHost {
        state: Mutex::new(FakeHostState {
            token: 42,
            sent: Vec::new(),
        }),
    });

    collomatique_python_runner::run_python_repl(Some(host.clone()), None, io)
        .expect("the session ends when the lines run out");

    assert_eq!(host.sent(), vec![Some(42)]);
}
