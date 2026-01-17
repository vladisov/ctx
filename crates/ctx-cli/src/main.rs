mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;
use config::Config;
use ctx_sources::Denylist;
use ctx_storage::Storage;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = cli::Cli::parse();
    let config = Config::load()?;

    let db_path = cli.data_dir.as_ref().map(|dir| dir.join("state.db"));
    let storage = Storage::new(db_path).await?;
    let denylist = Denylist::new(config.denylist.patterns.clone());

    match cli.command {
        // Quick workflow
        cli::Commands::Quick {
            file,
            output,
            max_related,
        } => commands::pack::quick(&storage, &denylist, file, output, max_related).await,

        // Pack management
        cli::Commands::Create { name, tokens } => {
            let budget = tokens.unwrap_or(config.budget_tokens);
            commands::pack::create(&storage, name, budget).await
        }
        cli::Commands::Add {
            pack,
            source,
            priority,
            start,
            end,
            max_files,
            exclude,
            recursive,
            with_related,
            related_max,
        } => {
            commands::pack::add(
                &storage,
                &denylist,
                pack,
                source,
                priority,
                start,
                end,
                max_files,
                exclude,
                recursive,
                with_related,
                related_max,
            )
            .await
        }
        cli::Commands::Rm { pack, artifact_id } => {
            commands::pack::remove(&storage, pack, artifact_id).await
        }
        cli::Commands::Ls => commands::pack::list(&storage).await,
        cli::Commands::Show { pack } => commands::pack::show(&storage, pack).await,
        cli::Commands::Preview {
            pack,
            tokens,
            redactions,
            payload,
        } => commands::pack::preview(&storage, pack, tokens, redactions, payload).await,
        cli::Commands::Cp { pack } => commands::pack::copy_to_clipboard(&storage, pack).await,
        cli::Commands::Delete { pack, force } => {
            commands::pack::delete(&storage, pack, force).await
        }
        cli::Commands::Lint { pack, fix } => {
            commands::pack::lint(&storage, &denylist, pack, fix).await
        }

        // Discovery
        cli::Commands::Suggest { file, max, format } => {
            commands::suggest::handle_suggest(file, max, &format).await
        }

        // Project config
        cli::Commands::Init { import } => commands::init::handle(&storage, import).await,
        cli::Commands::Sync => commands::pack::sync(&storage, &config, &denylist).await,
        cli::Commands::Save { packs, all } => commands::pack::save(&storage, packs, all).await,

        // Services
        cli::Commands::Mcp {
            stdio,
            port,
            host,
            read_only,
            tunnel,
        } => {
            let read_only = read_only || config.mcp.read_only;
            if stdio {
                commands::mcp::handle_stdio(&storage, read_only).await
            } else {
                let port = port.unwrap_or(config.mcp.port);
                let host = host.unwrap_or(config.mcp.host);
                commands::mcp::handle(&storage, host, port, read_only, tunnel).await
            }
        }
        cli::Commands::Ui { web, port } => {
            if web {
                commands::web::handle(port, cli.data_dir.as_deref()).await
            } else {
                commands::ui::handle(&storage).await
            }
        }

        // Shell completions
        cli::Commands::Completions { shell } => {
            cli::Cli::print_completions(shell);
            Ok(())
        }
    }
}
