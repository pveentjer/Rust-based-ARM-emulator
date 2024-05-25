use std::process::exit;
use std::rc::Rc;
use lalrpop_util::lalrpop_mod;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::cpu::{CPU, load_cpu_config};
use crate::loader::loader::{load, LoadError};

mod cpu;
mod loader;
mod frontend;
mod backend;
mod instructions;
mod memory_subsystem;


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
    let load_result = load(cpu_config.clone(), path);
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
            }
        }
    };

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(&program);
}
