use std::process::exit;
use std::rc::Rc;
use lalrpop_util::lalrpop_mod;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::cpu::{CPU, load_cpu_config};
use crate::loader::loader::{load_from_file, LoadError};

mod cpu;
mod loader;
mod frontend;
mod backend;
mod instructions;
mod memory_subsystem;
mod cpu_tests;


lalrpop_mod!(pub assembly, "/loader/assembly.rs");

#[derive(StructOpt, Debug)]
#[structopt(name = "ARM CPU Emulator")]
struct Opt {
    /// Path of the file to load
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,

    /// Sets a custom config file
    #[structopt(short, long, parse(from_os_str), default_value = "cpu.yaml")]
    config: PathBuf,

    #[structopt(short, long)]
    stats: bool,
}

fn main() {
    let opt = Opt::from_args();

    let cpu_config_path = opt.config.to_str().unwrap();
    let cpu_config = match load_cpu_config(cpu_config_path) {
        Ok(config) => config,
        Err(error) => {
            println!("Failed to load {}. Cause: {}", cpu_config_path, error);
            exit(0);
        }
    };

    let path = opt.file.to_str().unwrap();
    println!("Loading {}", path);
    let load_result = load_from_file(cpu_config.clone(), path);
    let program = match load_result {
        Ok(p) => Rc::new(p),
        Err(err) => {
            println!("Loading program '{}' failed.", path);
            match err {
                LoadError::ParseError(msg) => {
                    println!("{}", msg);
                    exit(1);
                }

                LoadError::AnalysisError(msg_vec) => {
                    for msg in msg_vec {
                        println!("{}", msg);
                    }
                    exit(1);
                }
                LoadError::NotFoundError(msg) => {
                    println!("{}", msg);
                    exit(1);
                }
                LoadError::IOError(msg) => {
                    println!("{}", msg);
                    exit(1);
                }
            }
        }
    };

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(&program);

    if opt.stats {
        show_stats(&mut cpu);
    }
}

fn show_stats(cpu: &CPU) {
    let perf_counters = cpu.perf_counters.borrow();

    let branch_total = perf_counters.branch_miss_prediction_cnt + perf_counters.branch_good_predictions_cnt;

    let ipc = perf_counters.retired_cnt as f32 / perf_counters.cycle_cnt as f32;

    let branch_prediction = if branch_total != 0 {
        100.0 * perf_counters.branch_good_predictions_cnt as f32 / branch_total as f32
    } else {
        0.0
    };

    println!("-------------------- [ stats ] -------------------------");
    println!("ipc {:.2}", ipc);
    println!("branch pred {:.2}%", branch_prediction);
    println!("branch miss prediction cnt: {}", perf_counters.branch_miss_prediction_cnt);
    println!("branch good predictions cnt: {}", perf_counters.branch_good_predictions_cnt);
    println!("decode cnt: {}", perf_counters.decode_cnt);
    println!("issue cnt: {}", perf_counters.issue_cnt);
    println!("dispatch cnt: {}", perf_counters.dispatch_cnt);
    println!("execute cnt: {}", perf_counters.execute_cnt);
    println!("retired cnt: {}", perf_counters.retired_cnt);
    println!("cycle cnt: {}", perf_counters.cycle_cnt);
    println!("bad speculation cnt: {}", perf_counters.bad_speculation_cnt);
    println!("pipeline flushes: {}", perf_counters.pipeline_flushes);
}
