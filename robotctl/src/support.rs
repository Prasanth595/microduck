//! Read-only support bundles.
//!
//! A failed robot frequently cannot run the app, and its journal can disappear after a power
//! loss. This command puts the durable update record beside a fresh health and version report,
//! without reading secrets or changing the robot. Journal excerpts remain an explicit choice:
//! logs may contain user context, so collecting them must not be a surprise side effect.

use std::fs;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use duck_ipc_proto as proto;

use crate::{
    Client, Failure, collect_health_report, collect_version_report, exit, result_of, unix_now,
};

const SCHEMA_VERSION: u32 = 1;
const LOG_UNITS: &[&str] = &[
    "robotd", "updaterd", "configd", "btd", "padd", "mediad", "tofd",
];

#[derive(Subcommand, Debug)]
pub(super) enum SupportCommand {
    /// Write a new directory containing a point-in-time diagnostic report.
    Bundle {
        /// A new directory to create. It is never overwritten.
        output: PathBuf,
        /// Include raw journal tails. Review these files before sending the bundle to anyone.
        #[arg(long)]
        include_logs: bool,
        /// Lines from each service when --include-logs is set.
        #[arg(long, default_value_t = 200, requires = "include_logs")]
        log_lines: usize,
    },
}

#[derive(serde::Serialize)]
struct Manifest {
    schema_version: u32,
    collected_at: i64,
    files: Vec<String>,
    failures: Vec<String>,
    includes_raw_logs: bool,
}

pub(super) fn run(
    updater_socket: &Path,
    robot_socket: &Path,
    config_socket: &Path,
    command: SupportCommand,
) -> Result<(), Failure> {
    match command {
        SupportCommand::Bundle {
            output,
            include_logs,
            log_lines,
        } => bundle(
            updater_socket,
            robot_socket,
            config_socket,
            &output,
            include_logs,
            log_lines,
        ),
    }
}

fn bundle(
    updater_socket: &Path,
    robot_socket: &Path,
    config_socket: &Path,
    output: &Path,
    include_logs: bool,
    log_lines: usize,
) -> Result<(), Failure> {
    if output.exists() {
        return Err(Failure::new(
            exit::USAGE,
            format!(
                "{} already exists; support bundles are never overwritten",
                output.display()
            ),
        ));
    }
    fs::create_dir(output).map_err(|error| {
        Failure::new(
            exit::FAILED,
            format!("could not create {}: {error}", output.display()),
        )
    })?;

    let mut files = Vec::new();
    let mut failures = Vec::new();
    let health = collect_health_report(updater_socket, robot_socket, config_socket);
    write_json(output, "health.json", &health, &mut files, &mut failures);
    let version = collect_version_report(updater_socket, robot_socket, config_socket);
    write_json(output, "version.json", &version, &mut files, &mut failures);

    collect_update_data(updater_socket, output, &mut files, &mut failures);
    if include_logs {
        collect_logs(config_socket, output, log_lines, &mut files, &mut failures);
    }

    let readme = if include_logs {
        "Microduck support bundle\n\nContains raw journal excerpts. Review logs/ before sharing: journal entries can include user context. Credentials, Wi-Fi configuration and account tokens are not collected by this command.\n"
    } else {
        "Microduck support bundle\n\nContains health, version and durable update history. Credentials, Wi-Fi configuration, account tokens and journal excerpts are not collected. Re-run with --include-logs only after reviewing the privacy warning.\n"
    };
    write_bytes(
        output,
        "README.txt",
        readme.as_bytes(),
        &mut files,
        &mut failures,
    );
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        collected_at: unix_now(),
        files,
        failures,
        includes_raw_logs: include_logs,
    };
    let manifest_path = output.join("manifest.json");
    let body = serde_json::to_vec_pretty(&manifest).map_err(|error| {
        Failure::new(exit::FAILED, format!("could not encode manifest: {error}"))
    })?;
    fs::write(&manifest_path, body).map_err(|error| {
        Failure::new(
            exit::FAILED,
            format!("could not write {}: {error}", manifest_path.display()),
        )
    })?;

    println!("support bundle written to {}", output.display());
    if !manifest.failures.is_empty() {
        eprintln!(
            "note: {} item(s) could not be collected; see manifest.json",
            manifest.failures.len()
        );
    }
    Ok(())
}

fn collect_update_data(
    socket: &Path,
    output: &Path,
    files: &mut Vec<String>,
    failures: &mut Vec<String>,
) {
    let mut client = match Client::connect(socket).and_then(|mut client| {
        client.hello()?;
        Ok(client)
    }) {
        Ok(client) => client,
        Err(error) => {
            failures.push(format!("update data: {}", error.message));
            return;
        }
    };
    match client.call(&proto::Call::Status).and_then(result_of) {
        Ok(status) => write_json(output, "updates-status.json", &status, files, failures),
        Err(error) => failures.push(format!("update status: {}", error.message)),
    }
    match client
        .call(&proto::Call::Log(proto::LogParams { limit: 50 }))
        .and_then(result_of)
    {
        Ok(log) => write_json(output, "updates-log.json", &log, files, failures),
        Err(error) => failures.push(format!("update log: {}", error.message)),
    }
}

fn collect_logs(
    socket: &Path,
    output: &Path,
    lines: usize,
    files: &mut Vec<String>,
    failures: &mut Vec<String>,
) {
    let logs_dir = output.join("logs");
    if let Err(error) = fs::create_dir(&logs_dir) {
        failures.push(format!("logs directory: {error}"));
        return;
    }
    for unit in LOG_UNITS {
        let result = Client::connect_to("configd", socket)
            .and_then(|mut client| {
                client.hello()?;
                client.call(&proto::Call::SystemLogs(proto::LogsParams {
                    unit: (*unit).to_owned(),
                    lines,
                    boot: 0,
                }))
            })
            .and_then(result_of);
        match result {
            Ok(log) => write_json(output, &format!("logs/{unit}.json"), &log, files, failures),
            Err(error) => failures.push(format!("logs for {unit}: {}", error.message)),
        }
    }
}

fn write_json(
    output: &Path,
    name: &str,
    value: &impl serde::Serialize,
    files: &mut Vec<String>,
    failures: &mut Vec<String>,
) {
    match serde_json::to_vec_pretty(value) {
        Ok(body) => write_bytes(output, name, &body, files, failures),
        Err(error) => failures.push(format!("{name}: could not encode JSON: {error}")),
    }
}

fn write_bytes(
    output: &Path,
    name: &str,
    body: &[u8],
    files: &mut Vec<String>,
    failures: &mut Vec<String>,
) {
    let path = output.join(name);
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        failures.push(format!("{name}: could not create directory: {error}"));
        return;
    }
    match fs::write(&path, body) {
        Ok(()) => files.push(name.to_owned()),
        Err(error) => failures.push(format!("{name}: could not write: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{LOG_UNITS, SupportCommand, bundle};
    use clap::Parser;

    use crate::{Cli, Namespace};

    #[test]
    fn bundle_logs_are_opt_in() {
        let cli = Cli::try_parse_from(["robotctl", "support", "bundle", "report"])
            .expect("support bundle must parse without raw logs");
        let Namespace::Support {
            command:
                SupportCommand::Bundle {
                    include_logs,
                    log_lines,
                    ..
                },
        } = cli.namespace
        else {
            panic!("expected support bundle");
        };
        assert!(!include_logs);
        assert_eq!(log_lines, 200);
    }

    #[test]
    fn bundle_collects_only_known_service_logs() {
        assert_eq!(
            LOG_UNITS,
            [
                "robotd", "updaterd", "configd", "btd", "padd", "mediad", "tofd"
            ]
        );
    }

    #[test]
    fn bundle_refuses_to_overwrite_an_existing_directory_before_contacting_a_daemon() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let error = bundle(
            Path::new("not-used.sock"),
            Path::new("not-used.sock"),
            Path::new("not-used.sock"),
            temp.path(),
            false,
            200,
        )
        .expect_err("an existing directory must not be overwritten");
        assert_eq!(error.code, crate::exit::USAGE);
    }
}
