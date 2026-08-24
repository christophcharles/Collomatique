// Replay a model dumped by `COLLO_CBC_DUMP_MODEL` through our own event
// handler, and print what comes back.
//
//     COLLO_CBC_DEBUG_EVENTS=1 cargo run -p collo-cbc --example replay -- \
//         dump-12345-008.collomodel [--mip-start dump-12345-008.collomipstart] [--log 1]
//
// The two diagnostics answer different halves of the same question:
// `COLLO_CBC_DEBUG_EVENTS` prints every *raw* CBC event before any filtering,
// while this example prints the progress events that actually reach a consumer.
// Run both together to see what we are discarding.

use std::path::PathBuf;
use std::process::ExitCode;

use collo_cbc::{EventType, IncumbentEvent, Model, ProblemDesc};

const USAGE: &str = "usage: replay <model.collomodel> [--mip-start <file>] [--log <level>]";

struct Args {
    model: PathBuf,
    mip_start: Option<PathBuf>,
    log_level: i32,
}

fn parse_args() -> Result<Args, String> {
    let mut model: Option<PathBuf> = None;
    let mut mip_start: Option<PathBuf> = None;
    let mut log_level = 0;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mip-start" => {
                let path = args.next().ok_or("--mip-start needs a file")?;
                mip_start = Some(PathBuf::from(path));
            }
            "--log" => {
                let level = args.next().ok_or("--log needs a level")?;
                log_level = level
                    .parse::<i32>()
                    .map_err(|_| format!("--log expects a number, got `{level}`"))?;
            }
            "-h" | "--help" => return Err(USAGE.to_string()),
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            other => {
                if model.is_some() {
                    return Err("only one model file can be replayed at a time".to_string());
                }
                model = Some(PathBuf::from(other));
            }
        }
    }

    Ok(Args {
        model: model.ok_or_else(|| USAGE.to_string())?,
        mip_start,
        log_level,
    })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };

    let desc = match ProblemDesc::read_from(&args.model) {
        Ok(desc) => desc,
        Err(e) => {
            eprintln!("could not read {}: {e}", args.model.display());
            return ExitCode::FAILURE;
        }
    };
    println!(
        "loaded {}: {} cols, {} rows, {} non-zeros, sense {}",
        args.model.display(),
        desc.num_cols,
        desc.num_rows,
        desc.mat_value.len(),
        if desc.obj_sense < 0 {
            "maximize"
        } else {
            "minimize"
        },
    );

    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(args.log_level);

    if let Some(path) = &args.mip_start {
        match collo_cbc::read_mip_start(path) {
            Ok(values) => {
                println!("mip start from {}: {} values", path.display(), values.len());
                model.set_mip_start(&values);
            }
            Err(e) => {
                eprintln!("could not read {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    let mut events = 0usize;
    let result = model.solve_with_callback(|p| {
        events += 1;
        let event = match p.event_type {
            EventType::Solution => "solution",
            EventType::TreeStatus => "treeStatus",
            EventType::Tick => {
                // A tick carries no numbers at all — printing zeros for the
                // bound and the node count would read as real ones.
                println!("event {events}: tick");
                return true;
            }
        };
        let incumbent = match &p.incumbent {
            IncumbentEvent::None => "none".to_string(),
            IncumbentEvent::Reconstructed { objective, .. } => format!("obj={objective}"),
            IncumbentEvent::ReconstructionFailed => "FAILED".to_string(),
        };
        println!(
            "event {events}: {event} bound={} nodes={} solutions={} incumbent={incumbent}",
            p.best_bound, p.node_count, p.solutions_found,
        );
        true
    });

    println!(
        "status={:?} obj={} bound={} nodes={} events={events}",
        result.status, result.obj_value, result.best_bound, result.node_count,
    );
    ExitCode::SUCCESS
}
