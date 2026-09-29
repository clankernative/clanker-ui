use clanker_ui::app_sink;
use clanker_ui::application::Application;
use clanker_ui::directory_sink::DirectorySink;
use clanker_ui::local::LocalPackage;
use clanker_ui::native::NativeAdapter;
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "clanker-ui",
    about = "Local, contract-checked UI component discovery and staging"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Pin a local package. The path is relative to the lock file's directory.
    Lock {
        #[arg(long)]
        lock: PathBuf,
        #[arg(long)]
        package: String,
        #[arg(long)]
        update: bool,
    },
    /// List ready components from the locked package.
    List {
        #[arg(long)]
        lock: PathBuf,
    },
    /// Search the locked catalog by word or phrase (not semantic ranking).
    Find {
        query: String,
        #[arg(long)]
        lock: PathBuf,
        #[arg(long, default_value = "native-html")]
        target: String,
    },
    /// Read a component's complete contract and agent guidance.
    Describe {
        name: String,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Show the selected component and its dependencies in build order.
    Graph {
        name: String,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Check component contracts, tokens and rendering fixtures.
    Verify {
        #[arg(long)]
        lock: PathBuf,
    },
    /// Compose checked app-owned button instances; preview unless --apply is set.
    Compose {
        #[arg(long)]
        lock: PathBuf,
        #[arg(long)]
        instances: PathBuf,
        #[arg(long)]
        base_css: PathBuf,
        #[arg(long)]
        theme: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Install managed outputs into an existing app rather than the standalone out directory.
        #[arg(long)]
        app: Option<PathBuf>,
        #[arg(long)]
        apply: bool,
    },
    /// Legacy single-button preview; use compose for app integration.
    Build {
        name: String,
        #[arg(long)]
        lock: PathBuf,
        #[arg(long)]
        theme: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "primary")]
        variant: String,
        #[arg(long)]
        apply: bool,
    },
}

fn run(cli: Cli) -> Result<(&'static str, Value), String> {
    let app = Application {
        source: LocalPackage,
        output: DirectorySink,
    };
    match cli.command {
        Command::Lock {
            lock,
            package,
            update,
        } => {
            let data = app.lock(&lock, &package, update)?;
            Ok(("lock", json!(data)))
        }
        Command::List { lock } => {
            let package = app.load(&lock)?;
            Ok((
                "list",
                json!({
                    "package": package.catalog.package.name,
                    "digest": package.digest,
                    "components": package.catalog.components().map(|c| json!({
                        "id": c.id(&package.catalog.package.name), "category": c.category,
                        "summary": c.summary, "target": c.target
                    })).collect::<Vec<_>>()
                }),
            ))
        }
        Command::Find {
            query,
            lock,
            target,
        } => {
            let package = app.load(&lock)?;
            let matches = app.find(&package, &query, &target);
            Ok((
                "find",
                json!({
                    "query": query, "target": target, "package": package.catalog.package.name,
                    "matches": matches.iter().map(|c| json!({
                        "id": c.id(&package.catalog.package.name), "summary": c.summary,
                        "useWhen": c.agent.use_when, "avoidWhen": c.agent.avoid_when,
                    })).collect::<Vec<_>>()
                }),
            ))
        }
        Command::Describe { name, lock } => {
            let package = app.load(&lock)?;
            let component = package
                .catalog
                .get(&name)
                .ok_or_else(|| format!("unknown or unready component: {name}"))?;
            Ok((
                "describe",
                json!({
                    "id": component.id(&package.catalog.package.name), "component": component,
                    "packageDigest": package.digest,
                }),
            ))
        }
        Command::Graph { name, lock } => {
            let package = app.load(&lock)?;
            let closure = package.catalog.resolve(&name)?;
            Ok((
                "graph",
                json!({
                    "components": closure.iter().map(|c| c.id(&package.catalog.package.name)).collect::<Vec<_>>(),
                    "assets": closure.iter().flat_map(|c| {
                        c.assets.scripts.iter().chain([&c.assets.template, &c.assets.styles])
                    }).collect::<std::collections::BTreeSet<_>>(),
                }),
            ))
        }
        Command::Verify { lock } => {
            let package = app.load(&lock)?;
            NativeAdapter::verify(&app.source, &package)?;
            Ok((
                "verify",
                json!({
                    "package": package.catalog.package.name, "digest": package.digest,
                    "verifiedComponents": package.catalog.components().map(|c| c.name.as_str()).collect::<Vec<_>>()
                }),
            ))
        }
        Command::Compose {
            lock,
            instances,
            base_css,
            theme,
            out,
            app: app_root,
            apply,
        } => {
            let bundle = app.compose(
                &lock,
                &instances,
                &base_css,
                &theme,
                &out,
                apply && app_root.is_none(),
            )?;
            let installed = if apply {
                app_root
                    .as_ref()
                    .map(|root| app_sink::install(root, &bundle))
                    .transpose()?
            } else {
                None
            };
            Ok((
                "compose",
                json!({
                    "applied": apply, "output": if app_root.is_some() { json!(app_root) } else { json!(out) },
                    "packageDigest": bundle.package_digest, "installed": installed,
                    "components": bundle.components, "files": bundle.files.keys().collect::<Vec<_>>()
                }),
            ))
        }
        Command::Build {
            name,
            lock,
            theme,
            out,
            variant,
            apply,
        } => {
            let bundle = app.build(&lock, &name, &variant, &theme, &out, apply)?;
            Ok((
                "build",
                json!({
                    "applied": apply, "output": out, "packageDigest": bundle.package_digest,
                    "components": bundle.components,
                    "files": bundle.files.keys().collect::<Vec<_>>()
                }),
            ))
        }
    }
}

fn main() {
    match run(Cli::parse()) {
        Ok((command, data)) => {
            println!(
                "{}",
                json!({"schemaVersion": 1, "command": command, "ok": true, "data": data, "diagnostics": []})
            );
        }
        Err(message) => {
            println!(
                "{}",
                json!({"schemaVersion": 1, "ok": false, "diagnostics": [{"code": "CUI001", "severity": "error", "message": message}]})
            );
            std::process::exit(1);
        }
    }
}
