use std::{
    io::Write,
    process::{Command, Stdio},
};

const SINGLETON: &[u8] = br#"{"abi":1,"renderer":"echarts_chart_v1","data":{"width":640,"height":320,"start":0,"end":10001,"y_min":0,"y_max":100,"title":"Protocol","kind":"line","samples":[{"time":5000,"value":42,"missing":false,"key":"only"}]}}"#;
const UNKNOWN: &[u8] = br#"{"abi":1,"renderer":"echarts_chart_v1","data":{"width":640,"height":320,"start":0,"end":10001,"y_min":0,"y_max":100,"title":"Protocol","kind":"line","samples":[],"unknown":true}}"#;
const MALFORMED: &[u8] = br#"{"abi":1,,}"#;

fn recorded_reply(request: &[u8]) -> &'static [u8] {
    if request == SINGLETON {
        include_bytes!("fixtures/recorded_singleton_response.json")
    } else if request == UNKNOWN {
        include_bytes!("fixtures/recorded_unknown_field_error.json")
    } else if request == MALFORMED {
        include_bytes!("fixtures/recorded_malformed_error.json")
    } else {
        panic!("vector is not present in deterministic recorded simulator")
    }
}

fn process_exchange_with_env(vectors: &[&[u8]], environment: &[(&str, &str)]) -> Vec<Vec<u8>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_clanker-chart-worker"))
        .envs(environment.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch actual worker executable");
    {
        let mut stdin = child.stdin.take().expect("worker stdin");
        for request in vectors {
            stdin.write_all(request).unwrap();
            stdin.write_all(b"\n").unwrap();
        }
    }
    let output = child.wait_with_output().expect("wait for worker process");
    assert!(
        output.status.success(),
        "worker stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<Vec<u8>> = output
        .stdout
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(<[u8]>::to_vec)
        .collect();
    assert_eq!(lines.len(), vectors.len());
    lines
}

fn process_exchange(vectors: &[&[u8]]) -> Vec<Vec<u8>> {
    process_exchange_with_env(vectors, &[])
}

#[test]
fn real_adapter_output_is_independent_of_host_timezone_and_locale() {
    let vectors: [&[u8]; 3] = [SINGLETON, UNKNOWN, SINGLETON];
    let expected =
        process_exchange_with_env(&vectors, &[("TZ", "UTC"), ("LANG", "C"), ("LC_ALL", "C")]);
    for timezone in ["America/Los_Angeles", "Europe/Berlin", "Asia/Tokyo"] {
        assert_eq!(
            process_exchange_with_env(
                &vectors,
                &[
                    ("TZ", timezone),
                    ("LANG", "de_DE.UTF-8"),
                    ("LC_ALL", "de_DE.UTF-8")
                ]
            ),
            expected,
            "timezone={timezone}"
        );
    }
}

#[test]
fn published_contract_matches_locked_component_metadata_without_executable_authority() {
    let output = Command::new(env!("CARGO_BIN_EXE_clanker-chart-worker"))
        .arg("--contracts")
        .output()
        .expect("read actual worker contracts");
    assert!(output.status.success());
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let locked: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../packages/vanilla/components/chart/renderer-contract.json"
    ))
    .unwrap();
    assert_eq!(actual, locked);
    assert_eq!(actual["schemaVersion"], 1);
    assert!(actual.get("executable").is_none());
    assert!(actual.get("digest").is_none());
    assert!(actual.get("abi").is_none());
}

#[test]
fn same_vectors_replay_and_interleave_against_real_process_and_recording() {
    let vectors: [&[u8]; 7] = [
        SINGLETON, MALFORMED, UNKNOWN, SINGLETON, UNKNOWN, MALFORMED, SINGLETON,
    ];
    let recorded: Vec<Vec<u8>> = vectors
        .iter()
        .map(|v| {
            recorded_reply(v)
                .strip_suffix(b"\n")
                .unwrap_or(recorded_reply(v))
                .to_vec()
        })
        .collect();

    // This invokes the actual worker process and ECharts/QuickJS. It verifies JSONL exchange and
    // deterministic fresh-job replay; it does not establish OS sandboxing or Native confinement.
    let actual = process_exchange(&vectors);
    assert_eq!(actual, recorded);
    assert_eq!(process_exchange(&vectors), actual);

    // The recorded simulator receives the identical byte vectors and replays checked-in frames.
    // This is a protocol fixture test, not a renderer implementation or process-confinement test.
    let simulated: Vec<Vec<u8>> = vectors
        .iter()
        .map(|v| {
            recorded_reply(v)
                .strip_suffix(b"\n")
                .unwrap_or(recorded_reply(v))
                .to_vec()
        })
        .collect();
    assert_eq!(simulated, actual);
    let singleton: serde_json::Value = serde_json::from_slice(&actual[0]).unwrap();
    assert_eq!(singleton["result"]["points"][0]["key"], "only");
    assert_eq!(
        singleton["result"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["role"] == 4)
            .unwrap()["stroke_width"],
        "0.3333333333333333"
    );
    assert_eq!(actual[1], actual[5]);
    assert_eq!(actual[2], actual[4]);
}
