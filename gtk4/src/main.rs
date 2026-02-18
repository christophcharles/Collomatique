//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Collomatique gtk4 UI
struct Args {
    /// Ignore all other parameters and run the python engine
    #[arg(long, default_value_t = false)]
    rpc_engine: bool,

    /// Open Collomatique directly editing a new colloscope
    #[arg(short, long, default_value_t = false)]
    new: bool,

    /// Pass a file as argument to open it with Collomatique
    file: Option<PathBuf>,

    /// Export file to SQLite database instead of opening GUI
    #[arg(long, value_name = "OUTPUT")]
    export: Option<PathBuf>,

    /// Export file to XLSX spreadsheet instead of opening GUI
    #[arg(long, value_name = "OUTPUT")]
    xlsx: Option<PathBuf>,

    /// Everything after gets passed through to GTK.
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    gtk_options: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if args.rpc_engine {
        return collomatique_rpc_engine::run_rpc_engine();
    }

    if let Some(export_path) = args.export {
        let input_file = args
            .file
            .ok_or_else(|| anyhow::anyhow!("--export requires an input file argument"))?;

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (data, _caveats) = collomatique_storage::load_data_from_file(&input_file)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to load file: {:?}", e))?;

            let pool = sqlx::SqlitePool::connect(":memory:").await?;
            collomatique_sqlite_state::create_schema(&pool).await?;

            collomatique_sqlite_state::inner_data_to_sqlite(&pool, data.get_inner_data()).await?;

            collomatique_sqlite_state::export_to_file(&pool, &export_path).await?;

            println!("Exported to {}", export_path.display());
            Ok::<(), anyhow::Error>(())
        })?;

        return Ok(());
    }

    if let Some(xlsx_path) = args.xlsx {
        let input_file = args
            .file
            .ok_or_else(|| anyhow::anyhow!("--xlsx requires an input file argument"))?;

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (data, _caveats) = collomatique_storage::load_data_from_file(&input_file)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to load file: {:?}", e))?;

            let pool = sqlx::SqlitePool::connect(":memory:").await?;
            collomatique_sqlite_state::create_schema(&pool).await?;
            collomatique_sqlite_state::inner_data_to_sqlite(&pool, data.get_inner_data()).await?;

            collomatique_xlsx::write_xlsx(&pool, &xlsx_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write XLSX: {}", e))?;

            println!("Exported to {}", xlsx_path.display());
            Ok::<(), anyhow::Error>(())
        })?;

        return Ok(());
    }

    let payload = collomatique_gtk4::AppInit {
        new: args.new,
        file_name: args.file,
    };

    let program_invocation = std::env::args().next().unwrap();
    let mut gtk_args = vec![program_invocation];
    gtk_args.extend(args.gtk_options.clone());

    let app = RelmApp::new("fr.collomatique.gtk4").with_args(gtk_args);
    app.allow_multiple_instances(true);
    app.run::<AppModel>(payload);

    Ok(())
}
