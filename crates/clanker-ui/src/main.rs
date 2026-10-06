use clanker_ui::app_sink;
use clanker_ui::application::Application;
use clanker_ui::directory_sink::DirectorySink;
use clanker_ui::local::LocalPackage;
use clanker_ui::native::NativeAdapter;
use clanker_ui::ports::PackageSource;
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
    /// Report the installed tool's supported contracts without reading an app.
    Capabilities,
    /// Assemble Native's private protocol-v1 request with binding ABI 2.
    Assemble {
        #[arg(long)]
        request: PathBuf,
    },
    /// Author the canonical Native UI lock (same schema as lock).
    NativeLock {
        #[arg(long)]
        lock: PathBuf,
        #[arg(long)]
        package: String,
        #[arg(long)]
        update: bool,
    },
    /// Write a local unsigned override pin; operator verification of executable trust remains required.
    NativePin {
        #[arg(long)]
        output: PathBuf,
    },
    /// Prepare a relocatable, unsigned Native tool and package bundle.
    NativeBundle {
        #[arg(long)]
        package: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        source_revision: String,
    },
    /// Verify a relocatable Native bundle without executing its binary.
    VerifyNativeBundle {
        #[arg(long)]
        bundle: PathBuf,
    },
    /// Expand locked package declarations without executing application operations.
    Expand {
        #[arg(long)]
        lock: PathBuf,
        #[arg(long)]
        ui: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Render a script-free static preview from an explicit fake-data scene.
    Render {
        #[arg(long)]
        lock: PathBuf,
        #[arg(long)]
        ui: PathBuf,
        #[arg(long)]
        scene: PathBuf,
        #[arg(long)]
        fragment: Option<PathBuf>,
    },
    /// Discover described tokens and their CSS-resolved baseline defaults.
    Tokens {
        #[arg(long)]
        lock: PathBuf,
        #[arg(long)]
        component: Option<String>,
    },
    /// Inspect authored CSS without changing files or running the application.
    CheckCss {
        #[arg(long)]
        ui: PathBuf,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Return bounded, locked component selection and verification context.
    Context {
        name: String,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Diagnose the locked package; does not build or mutate an application.
    Doctor {
        #[arg(long)]
        lock: PathBuf,
    },
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
    /// Search the locked icon catalog by name, label, or category.
    FindIcon {
        query: String,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Read a component's complete contract and agent guidance.
    Describe {
        name: String,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Export the typed property descriptions for the locked component catalog.
    Properties {
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

struct CommandResult {
    command: &'static str,
    data: Value,
    diagnostics: Vec<catalog_core::css_check::CssDiagnostic>,
    ok: bool,
}

fn run(cli: Cli) -> Result<CommandResult, String> {
    let app = Application {
        source: LocalPackage,
        output: DirectorySink,
    };
    let result: Result<(&'static str, Value), String> = match cli.command {
        Command::CheckCss { ui, lock } => {
            let package = app.load(&lock)?;
            let report = clanker_ui::theming::check_css(&ui, &package)?;
            let data = serde_json::to_value(&report).map_err(|error| error.to_string())?;
            return Ok(CommandResult {
                command: "check-css",
                data,
                ok: report.errors == 0,
                diagnostics: report.diagnostics,
            });
        }
        Command::Tokens { lock, component } => {
            let package = app.load(&lock)?;
            let mut catalog = clanker_ui::theming::token_catalog(&package)?;
            if let Some(name) = component {
                let selected = package
                    .catalog
                    .get(&name)
                    .ok_or_else(|| format!("unknown or unready component: {name}"))?;
                catalog.tokens.retain(|token| {
                    token.owner == name
                        || token.readers.contains(&name)
                        || selected.tokens.contains(&token.name)
                });
            }
            Ok((
                "tokens",
                json!({"packageDigest":package.digest,"tokens":catalog.tokens,"resolutionScope":"Locked baseline CSS and declared component fallbacks; not computed browser styles."}),
            ))
        }
        Command::Assemble { request } => Ok((
            "assemble",
            json!(clanker_ui::expand::assemble_request(&request)?),
        )),
        Command::NativeLock {
            lock,
            package,
            update,
        } => Ok((
            "native-lock",
            clanker_ui::expand::native_lock_with_update(&lock, &package, update)?,
        )),
        Command::NativePin { output } => {
            Ok(("native-pin", clanker_ui::expand::native_pin(&output)?))
        }
        Command::NativeBundle {
            package,
            output,
            source_revision,
        } => Ok((
            "native-bundle",
            clanker_ui::native_bundle::prepare(&package, &output, &source_revision)?,
        )),
        Command::VerifyNativeBundle { bundle } => Ok((
            "verify-native-bundle",
            clanker_ui::native_bundle::verify(&bundle)?,
        )),
        Command::Expand { lock, ui, out } => Ok((
            "expand",
            json!(clanker_ui::expand::expand(&lock, &ui, out.as_deref())?),
        )),
        Command::Render {
            lock,
            ui,
            scene,
            fragment,
        } => Ok((
            "render",
            json!(clanker_ui::preview::render(
                &lock,
                &ui,
                &scene,
                fragment.as_deref()
            )?),
        )),
        Command::Capabilities => Ok((
            "capabilities",
            json!({
                "toolVersion": env!("CARGO_PKG_VERSION"), "schemaVersion": 1,
                "target": "native-html", "networkRequired": false,
                "discovery": ["list", "find", "find-icon", "describe", "properties", "graph", "context", "tokens"],
                "validation": ["verify", "doctor", "check-css"],
                "theming": {"schemaVersion":1,"readOnly":true,"commands":["tokens","check-css"],"warningsFail":false,"unknownTokensFail":true,"componentDetailsField":"tokenDetails","analysis":"Static CSS and possible template structure; no JavaScript or computed cascade."},
                "assembly": ["assemble", "expand", "render"],
                "nativeBundle": {"commands":["native-bundle", "verify-native-bundle"],"schemaVersion":1,"ciArtifactTargets":["linux-x86_64"],"artifact":"CI workflow artifact only; no published release or install channel","trust":"Unsigned identity only; operator approval required."},
                "nativeLock": {"path":"ui/ui.lock.json","schemaVersion":1,"provider":"clanker-ui.native","commands":["lock","native-lock"],"updates":"Explicit --update only; no legacy lock schema."},
                "nativePin": "Explicit local unsigned override output only; operator verifies executable trust.",
                "integration": {"componentStatusField":"component.status","nativeStatusField":"component.integration.native","portsField":"component.integration.ports","portTypesField":"component.assets.contracts","missingHostMetadata":"No advertised host support; admission is independent.","readyMeaning":"Component-complete, not backend-integrated."},
                "adapterProtocol": 2,
                "bindingAbi": 2,
                "templateEngine": "minijinja-2.12.0",
                "lock": "Exact declared local package bytes; updates are explicit.",
                "ownership": "Native/app owns operations, state and resource admission. This tool checks package contracts, not host or browser readiness.",
                "unsupported": ["semantic search", "app initialization", "app upgrades", "host builds", "browser acceptance"]
            }),
        )),
        Command::Context { name, lock } => {
            let package = app.load(&lock)?;
            let component = package
                .catalog
                .get(&name)
                .ok_or_else(|| format!("unknown or unready component: {name}"))?;
            let closure = package.catalog.resolve(&name)?;
            // One declared fixture provides a narrow starting point. Other fixture
            // paths remain visible, rather than silently claiming full coverage.
            let example = component
                .fixtures
                .first()
                .map(|path| {
                    let bytes = app.source.asset(&package, path)?;
                    let mut value: Value = serde_json::from_slice(&bytes)
                        .map_err(|error| format!("{path}: {error}"))?;
                    if let Some(object) = value.as_object_mut() {
                        object.remove("expectedHtml");
                    }
                    Ok::<_, String>(json!({"path": path, "input": value}))
                })
                .transpose()?;
            Ok((
                "context",
                json!({
                    "id": component.id(&package.catalog.package.name),
                    "packageVersion": package.catalog.package.version,
                    "packageDigest": package.digest,
                    "component": clanker_ui::theming::described_component(&package, component)?,
                    "properties": catalog_core::properties::describe_component_properties(component)?,
                    "example": example,
                    "exampleSelection": "First declared fixture; consult component.fixtures for every state.",
                    "dependencies": closure.iter().map(|value| value.id(&package.catalog.package.name)).collect::<Vec<_>>(),
                    "verification": {"package": ["verify", "--lock", lock.to_string_lossy()],
                        "remaining": ["Native host admission/build", "keyboard and no-JavaScript behavior", "responsive browser review"]}
                }),
            ))
        }
        Command::Doctor { lock } => {
            let package = app.load(&lock)?;
            NativeAdapter::verify(&app.source, &package)?;
            Ok((
                "doctor",
                json!({
                    "package": package.catalog.package.name,
                    "version": package.catalog.package.version,
                    "digest": package.digest,
                    "checks": ["lock identity and declared bytes", "component dependency closure", "typed rendering fixtures", "declared theme tokens", "property catalog freshness", "authoring example expansion"],
                    "scope": "Locked package only; application integration and browser behavior are not checked."
                }),
            ))
        }
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
        Command::FindIcon { query, lock } => {
            let package = app.load(&lock)?;
            let geometries: std::collections::BTreeMap<String, String> =
                serde_json::from_slice(&app.source.asset(&package, "icons.json")?)
                    .map_err(|error| format!("icons.json: {error}"))?;
            let catalog = catalog_core::icon::IconCatalog::parse(
                &app.source.asset(&package, "icon-catalog.json")?,
                &geometries,
            )
            .map_err(|error| format!("icon-catalog.json: {error}"))?;
            Ok((
                "find-icon",
                json!({
                    "query": query,
                    "package": package.catalog.package.name,
                    "matches": catalog.find(&query).iter().map(|icon| json!({
                        "name": icon.name, "label": icon.label, "category": icon.category,
                        "categoryLabel": catalog.category_label(&icon.category),
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
            let icon_catalog = if name == "icon" {
                let geometries: std::collections::BTreeMap<String, String> =
                    serde_json::from_slice(&app.source.asset(&package, "icons.json")?)
                        .map_err(|error| format!("icons.json: {error}"))?;
                Some(
                    catalog_core::icon::IconCatalog::parse(
                        &app.source.asset(&package, "icon-catalog.json")?,
                        &geometries,
                    )
                    .map_err(|error| format!("icon-catalog.json: {error}"))?,
                )
            } else {
                None
            };
            Ok((
                "describe",
                json!({
                    "id": component.id(&package.catalog.package.name), "component": clanker_ui::theming::described_component(&package, component)?,
                    "packageDigest": package.digest, "iconCatalog": icon_catalog,
                    "properties": catalog_core::properties::describe_component_properties(component)?,
                }),
            ))
        }
        Command::Properties { lock } => {
            let package = app.load(&lock)?;
            let icons: std::collections::BTreeMap<String, String> =
                serde_json::from_slice(&app.source.asset(&package, "icons.json")?)
                    .map_err(|error| format!("icons.json: {error}"))?;
            Ok((
                "properties",
                NativeAdapter::property_catalog_with_icons(&package, &icons)?,
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
                        c.assets.scripts.iter().chain(c.assets.contracts.iter()).chain([&c.assets.template, &c.assets.styles])
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
    };
    let (command, data) = result?;
    Ok(CommandResult {
        command,
        data,
        diagnostics: vec![],
        ok: true,
    })
}

fn main() {
    match run(Cli::parse()) {
        Ok(result) => {
            println!(
                "{}",
                json!({"schemaVersion": 1, "command": result.command, "ok": result.ok, "data": result.data, "diagnostics": result.diagnostics})
            );
            if !result.ok {
                std::process::exit(1);
            }
        }
        Err(message) => {
            let envelope = if std::env::args().nth(1).as_deref() == Some("assemble") {
                json!({"schemaVersion": 1, "ok": false, "command": "assemble", "data": null, "diagnostics": [{"code": "CUI001", "severity": "error", "message": message}]})
            } else {
                json!({"schemaVersion": 1, "ok": false, "diagnostics": [{"code": "CUI001", "severity": "error", "message": message}]})
            };
            println!("{envelope}");
            std::process::exit(1);
        }
    }
}
