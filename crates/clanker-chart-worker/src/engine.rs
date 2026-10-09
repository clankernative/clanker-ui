use clanker_chart_worker::{Engine, EngineOutput, Input};
use rquickjs::{Context, Runtime};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

const ECHARTS: &str = include_str!("../vendor/echarts/echarts.min.js");
pub const ENGINE_SHA256: &str = "baa8dfe7e1d9336b98e8986ba7e20ea15e7cdbea1ef42a59d59478632fa45a1d";
const DEADLINE: Duration = Duration::from_secs(2);

pub struct Echarts;

impl Engine for Echarts {
    fn render(&self, input: &Input) -> Result<EngineOutput, &'static str> {
        run_engine(input)
    }
}

fn run_engine(i: &Input) -> Result<EngineOutput, &'static str> {
    let actual = format!("{:x}", Sha256::digest(ECHARTS.as_bytes()));
    if actual != ENGINE_SHA256 {
        return Err("engine_integrity");
    }
    let rt = Runtime::new().map_err(|_| "engine")?;
    rt.set_memory_limit(128 * 1024 * 1024);
    rt.set_max_stack_size(512 * 1024);
    let begin = Instant::now();
    rt.set_interrupt_handler(Some(Box::new(move || begin.elapsed() >= DEADLINE)));
    let ctx = Context::full(&rt).map_err(|_| "engine")?;
    let json = serde_json::to_string(i).map_err(|_| "input")?;
    let js_literal = serde_json::to_string(&json).map_err(|_| "input")?;
    let source = format!(
        r#"(()=>{{
let __timerId=0; const __timers=[];
globalThis.setTimeout=(fn,...args)=>{{if(__timers.length>=32)throw new Error('timer_limit');const id=++__timerId;__timers.push([id,fn,args]);return id}};
globalThis.clearTimeout=(id)=>{{const t=__timers.find(t=>t[0]===id);if(t)t[1]=null}};
globalThis.setInterval=globalThis.setTimeout; globalThis.clearInterval=globalThis.clearTimeout;
globalThis.requestAnimationFrame=(fn)=>globalThis.setTimeout(()=>fn(0)); globalThis.cancelAnimationFrame=globalThis.clearTimeout;
Math.random=()=>0.5;
const __HostDate=Date; function __FixedDate(...args){{if(!new.target)return new __HostDate(0).toISOString();return args.length?new __HostDate(...args):new __HostDate(0)}}
__FixedDate.prototype=__HostDate.prototype; Object.setPrototypeOf(__FixedDate,__HostDate); __FixedDate.now=()=>0; __FixedDate.parse=__HostDate.parse; __FixedDate.UTC=__HostDate.UTC; globalThis.Date=__FixedDate; Date.prototype.getTimezoneOffset=()=>0;
if(typeof process!=='undefined'||typeof require!=='undefined'||typeof fetch!=='undefined'||typeof document!=='undefined'||typeof window!=='undefined'||typeof XMLHttpRequest!=='undefined')throw new Error('host_api');
{echarts}
const data=JSON.parse({input}); const ec=echarts.init(null,null,{{renderer:'svg',ssr:true,width:data.width,height:data.height}});
const values=data.samples.map(s=>s.missing?null:[s.time,s.value]);
ec.setOption({{animation:false,useUTC:true,backgroundColor:'transparent',title:{{text:data.title,left:'center',textStyle:{{color:'#555d63',fontFamily:'Arial',fontSize:14}}}},grid:{{left:56,right:20,top:48,bottom:40,containLabel:false}},xAxis:{{type:'time',min:data.start,max:data.end,axisLine:{{lineStyle:{{color:'#555d63'}}}},axisTick:{{show:false}},splitLine:{{show:false}},axisLabel:{{color:'#555d63',fontFamily:'Arial',fontSize:11,hideOverlap:true,formatter:(v)=>{{const iso=new Date(v).toISOString(),span=data.end-data.start;return span<60000?iso.slice(11,23):span<3600000?iso.slice(11,19):span<=172800000?iso.slice(11,16):iso.slice(5,10)}}}}}},yAxis:{{type:'value',min:data.y_min,max:data.y_max,axisLine:{{show:false}},axisTick:{{show:false}},axisLabel:{{color:'#555d63',fontFamily:'Arial',fontSize:11}},splitLine:{{lineStyle:{{color:'#d7dcdf'}}}}}},series:[{{type:data.kind,data:values,connectNulls:false,showSymbol:true,symbol:'circle',symbolSize:6,lineStyle:{{color:'#a84716',width:2}},itemStyle:data.kind==='line'?{{color:'#fff',borderColor:'#a84716',borderWidth:1}}:{{color:'#a84716'}},barWidth:'55%'}}]}});
const pix=data.samples.map(s=>ec.convertToPixel({{xAxisIndex:0,yAxisIndex:0}},[s.time,s.missing?data.y_min:s.value])); const r=ec.getModel().getComponent('grid').coordinateSystem.getRect();
const svg=ec.renderToSVGString(); ec.dispose(); let n=0; while(__timers.length&&n++<32){{const t=__timers.shift();if(t[1])t[1](...t[2])}} if(__timers.length)throw new Error('timer_limit'); return JSON.stringify({{svg,rect:[r.x,r.y,r.width,r.height],pix}})}})()"#,
        echarts = ECHARTS,
        input = js_literal
    );
    let result: String = ctx
        .with(|ctx| ctx.eval(source.as_str()))
        .map_err(|_| "render")?;
    if begin.elapsed() > DEADLINE {
        return Err("deadline");
    }
    serde_json::from_str(&result).map_err(|_| "engine_result")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clanker_chart_worker::{render_with, validate, Plot, Sample, Scene};
    fn fixture(kind: &str) -> Input {
        Input {
            width: 640,
            height: 320,
            start: 0,
            end: 10_001,
            y_min: 0,
            y_max: 100,
            title: "Requests".into(),
            kind: kind.into(),
            samples: vec![
                Sample {
                    time: 0,
                    value: 0,
                    missing: false,
                    key: "zero".into(),
                },
                Sample {
                    time: 5_000,
                    value: 40,
                    missing: true,
                    key: "gap".into(),
                },
                Sample {
                    time: 10_000,
                    value: 100,
                    missing: false,
                    key: "peak".into(),
                },
            ],
        }
    }

    fn render(input: &Input) -> Result<Scene, &'static str> {
        render_with(input, &Echarts)
    }
    #[test]
    fn dynamic_line_gap_zero_and_coordinates() {
        let s = render(&fixture("line")).unwrap();
        assert_eq!(
            s.plot,
            Plot {
                x: "56".into(),
                y: "48".into(),
                width: "548.715".into(),
                height: "232".into()
            }
        );
        assert_eq!(s.points.len(), 3);
        assert_eq!(s.points[0].value, 0);
        assert!(!s.points[0].missing);
        assert!(s.points[1].missing);
        assert_eq!(s.points[0].x, "56");
        assert_eq!(s.points[2].x, "604.66");
        assert_eq!(s.points[0].y, "280");
        assert_eq!(s.points[2].y, "48");
        assert!(s
            .labels
            .iter()
            .any(|l| l.text == "0" && l.anchor == "end" && l.baseline == "central"));
        assert!(s
            .labels
            .iter()
            .any(|l| l.text == "Requests" && l.anchor == "middle" && l.baseline == "central"));
        assert!(s
            .paths
            .iter()
            .filter(|p| p.role == 2)
            .all(|p| !p.d.contains('L')));
        let mut full = fixture("line");
        full.samples.retain(|p| !p.missing);
        let full_scene = render(&full).unwrap();
        assert!(full_scene.paths.iter().any(|p| p.role == 2 && p.clipped));
        assert!(full_scene.paths.iter().any(|p| p.role == 4));
    }
    #[test]
    fn time_axis_format_switches_by_fixed_utc_range_and_hides_overlap() {
        let mut day = fixture("line");
        day.start = 0;
        day.end = 86_400_000;
        day.samples = vec![Sample {
            time: 0,
            value: 20,
            missing: false,
            key: "day".into(),
        }];
        let labels = render(&day).unwrap().labels;
        assert!(labels
            .iter()
            .filter(|l| l.font_size == "11px" && l.text.contains(':'))
            .all(|l| l.text.len() == 5 && l.text.matches(':').count() == 1));
        let mut two_days = day.clone();
        two_days.end = 172_800_000;
        assert!(render(&two_days)
            .unwrap()
            .labels
            .iter()
            .filter(|l| l.font_size == "11px" && l.text.contains(':'))
            .all(|l| l.text.len() == 5));

        let mut minute = day.clone();
        minute.end = 30_000;
        let labels = render(&minute).unwrap().labels;
        assert!(labels
            .iter()
            .filter(|l| l.font_size == "11px" && l.text.contains(':'))
            .all(|l| l.text.len() == 12 && l.text.ends_with(".000")));

        let mut short_hour = day;
        short_hour.end = 30 * 60 * 1000;
        let labels = render(&short_hour).unwrap().labels;
        assert!(labels
            .iter()
            .filter(|l| l.font_size == "11px" && l.text.contains(':'))
            .all(|l| l.text.len() == 8 && l.text.matches(':').count() == 2));
    }
    #[test]
    fn spaced_weekly_title_preserves_svg_space_and_source_fonts() {
        let i = Input {
            width: 640,
            height: 240,
            start: 1_735_689_600_000,
            end: 1_736_294_400_000,
            y_min: 0,
            y_max: 100,
            title: "Current samples".into(),
            kind: "line".into(),
            samples: vec![
                Sample {
                    time: 1_735_689_600_000,
                    value: 0,
                    missing: false,
                    key: "measured_zero".into(),
                },
                Sample {
                    time: 1_735_862_400_000,
                    value: 72,
                    missing: false,
                    key: "observed".into(),
                },
                Sample {
                    time: 1_736_035_200_000,
                    value: 0,
                    missing: true,
                    key: "gap".into(),
                },
                Sample {
                    time: 1_736_208_000_000,
                    value: 91,
                    missing: false,
                    key: "observed_end".into(),
                },
            ],
        };
        let scene = render(&i).unwrap();
        let title = scene
            .labels
            .iter()
            .find(|l| l.text == "Current samples")
            .unwrap();
        assert_eq!(title.font_size, "14px");
        assert_eq!(title.font_weight, "bold");
        assert_eq!(title.anchor, "middle");
        assert!(scene
            .labels
            .iter()
            .filter(|l| l.text != "Current samples")
            .all(|l| l.font_size == "11px" && l.font_weight == "normal"));
        assert!(scene
            .labels
            .iter()
            .any(|l| l.font_size == "11px" && l.text.contains('-')));
    }
    #[test]
    fn real_engine_handles_all_missing_singleton_and_gap_isolated_series() {
        let mut all_missing = fixture("line");
        for sample in &mut all_missing.samples {
            sample.missing = true;
            sample.value = 1_000;
        }
        assert!(validate(&all_missing).is_ok());
        let empty = render(&all_missing).unwrap();
        assert_eq!(empty.points.len(), 3);
        assert!(empty.points.iter().all(|p| p.missing && p.value == 1_000));
        assert!(!empty.paths.iter().any(|p| matches!(p.role, 2..=4)));

        let mut singleton = fixture("line");
        singleton.samples = vec![Sample {
            time: 5_000,
            value: 40,
            missing: false,
            key: "only".into(),
        }];
        let one = render(&singleton).unwrap();
        assert_eq!(one.points.len(), 1);
        assert!(one.paths.iter().any(|p| p.role == 4));
        assert!(one
            .paths
            .iter()
            .filter(|p| p.role == 2)
            .all(|p| !p.d.contains('L')));

        let mut isolated = fixture("line");
        isolated.samples = vec![
            Sample {
                time: 1_000,
                value: 1_000,
                missing: true,
                key: "before".into(),
            },
            Sample {
                time: 5_000,
                value: 40,
                missing: false,
                key: "island".into(),
            },
            Sample {
                time: 9_000,
                value: 1_000,
                missing: true,
                key: "after".into(),
            },
        ];
        let island = render(&isolated).unwrap();
        assert!(
            island.points[0].missing && island.points[1].value == 40 && island.points[2].missing
        );
        assert!(island.paths.iter().any(|p| p.role == 4));
        assert!(island
            .paths
            .iter()
            .filter(|p| p.role == 2)
            .all(|p| !p.d.contains('L')));
    }
    #[test]
    fn bar_range_and_changed_values_are_real_engine_output() {
        let a = render(&fixture("bar")).unwrap();
        let mut b = fixture("bar");
        b.start = 2_000;
        b.end = 8_001;
        b.samples = vec![
            Sample {
                time: 2_000,
                value: 10,
                missing: false,
                key: "x".into(),
            },
            Sample {
                time: 8_000,
                value: 80,
                missing: false,
                key: "y".into(),
            },
        ];
        let changed = render(&b).unwrap();
        assert_ne!(a.paths, changed.paths);
        assert_ne!(a.points[0].y, changed.points[0].y);
        assert!(a.paths.iter().any(|p| p.role == 3));
    }
    #[test]
    fn fresh_contexts_replay_across_interleaving() {
        let a = fixture("line");
        let expected = render(&a).unwrap();
        let _ = render(&fixture("bar")).unwrap();
        assert_eq!(expected, render(&a).unwrap());
    }
}
