use clanker_chart_worker::{process_with, scene_from_output, Engine, EngineOutput, Input, Sample};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::Cell;

// A deterministic engine-port model, not an ECharts implementation or confinement proof.
struct SimulatedEngine {
    outcome: Cell<u32>,
    calls: Cell<usize>,
}

fn output(input: &Input) -> EngineOutput {
    EngineOutput {
        svg: r#"<svg xmlns="http://www.w3.org/2000/svg" width="640"></svg>"#.into(),
        rect: [56.0, 48.0, 564.0, 232.0],
        pix: input
            .samples
            .iter()
            .map(|s| [56.0 + s.time as f64, 280.0 - s.value as f64])
            .collect(),
    }
}

impl Engine for SimulatedEngine {
    fn render(&self, input: &Input) -> Result<EngineOutput, &'static str> {
        self.calls.set(self.calls.get() + 1);
        match self.outcome.get() {
            0 => Ok(output(input)),
            1 => Err("deadline"),
            2 => Err("engine"),
            3 => {
                let mut v = output(input);
                v.pix[0][0] = f64::INFINITY;
                Ok(v)
            }
            _ => unreachable!(),
        }
    }
}

fn input(value: i64, missing: bool, kind: &str) -> Input {
    Input {
        width: 640,
        height: 320,
        start: 0,
        end: 101,
        y_min: 0,
        y_max: 100,
        title: "Scheduled chart".into(),
        kind: kind.into(),
        samples: vec![Sample {
            time: 10,
            value,
            missing,
            key: "sample".into(),
        }],
    }
}

fn replay(seed: u32) -> Vec<Vec<u8>> {
    let engine = SimulatedEngine {
        outcome: Cell::new(0),
        calls: Cell::new(0),
    };
    let mut random = seed;
    let mut transcript = Vec::new();
    let mut expected_calls = 0;
    for step in 0..64 {
        random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let outcome = (random >> 8) % 4;
        engine.outcome.set(outcome);
        let valid = step % 7 != 0;
        let mut data = input(
            i64::from(random % 101),
            random & 1 == 0,
            if random & 2 == 0 { "line" } else { "bar" },
        );
        if !valid {
            data.samples[0].time = data.end;
        }
        let frame = serde_json::to_vec(&json!({"abi":1,"renderer":"echarts_chart_v1","data":data}))
            .unwrap();
        let bytes = process_with(&frame, &engine);
        let reply: Value = serde_json::from_slice(&bytes).unwrap();
        let trace = format!("seed={seed} step={step} outcome={outcome} valid={valid}");
        assert_eq!(
            reply["inputDigest"],
            format!("sha256:{:x}", Sha256::digest(&frame)),
            "{trace}"
        );
        if !valid {
            assert_eq!(reply["error"], "sample_order", "{trace}");
        } else {
            expected_calls += 1;
            match outcome {
                0 => {
                    assert!(reply.get("error").is_none(), "{trace}");
                    let point = &reply["result"]["points"][0];
                    assert_eq!(point["value"], data.samples[0].value, "{trace}");
                    assert_eq!(point["missing"], data.samples[0].missing, "{trace}");
                    assert_eq!(point["key"], "sample", "{trace}");
                }
                1 => assert_eq!(reply["error"], "deadline", "{trace}"),
                2 => assert_eq!(reply["error"], "engine", "{trace}"),
                3 => assert_eq!(reply["error"], "svg_coordinate", "{trace}"),
                _ => unreachable!(),
            }
        }
        assert_eq!(
            engine.calls.get(),
            expected_calls,
            "{trace}: invalid input reached engine"
        );
        transcript.push(bytes);
    }
    transcript
}

#[test]
fn seeded_success_failure_and_invalid_input_schedules_replay_exactly() {
    for seed in 1..=64 {
        assert_eq!(replay(seed), replay(seed), "seed={seed}");
    }
}

#[test]
fn malformed_engine_coordinates_reject_without_panics_or_empty_fallbacks() {
    let data = input(0, false, "line");
    for pix in [
        json!([[]]),
        json!([[1]]),
        json!([[1, 2, 3]]),
        json!([[null, 2]]),
    ] {
        assert!(serde_json::from_value::<EngineOutput>(
            json!({"svg":"", "rect":[0,0,1,1], "pix":pix})
        )
        .is_err());
    }
    let mut short = output(&data);
    short.pix.clear();
    assert_eq!(scene_from_output(&data, short), Err("engine_result"));
    for invalid in [f64::NAN, f64::INFINITY, -1.0, 321.0, 100_001.0] {
        let mut bad = output(&data);
        bad.pix[0][1] = invalid;
        assert_eq!(scene_from_output(&data, bad), Err("svg_coordinate"));
    }
    for rect in [
        [0.0, 0.0, -1.0, 1.0],
        [0.0, 0.0, 641.0, 1.0],
        [f64::NAN, 0.0, 1.0, 1.0],
    ] {
        let mut bad = output(&data);
        bad.rect = rect;
        assert_eq!(scene_from_output(&data, bad), Err("plot_rect"));
    }
}
