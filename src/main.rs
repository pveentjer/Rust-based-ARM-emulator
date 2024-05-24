use std::error::Error;
use std::process::exit;
use std::rc::Rc;
use lalrpop_util::lalrpop_mod;

use crate::cpu::{CPU, load_cpu_config};
use crate::loader::loader::{load, LoadError};

mod cpu;
mod loader;
mod frontend;
mod backend;
mod instructions;
mod memory_subsystem;


lalrpop_mod!(pub assembly, "/loader/assembly.rs");

fn main() {
    let cpu_config_path = "cpu.yaml";
    let cpu_config = match load_cpu_config(cpu_config_path){
        Ok(config) => config,
        Err(error) => {
            println!("Failed to load {}. Cause:",error);
            exit(0);
        }
    };

    let path = "asm/load_store.asm";
    println!("Loading {}",path);
    let load_result = load(cpu_config.clone(), path);
    let program = match load_result {
        Ok(p) => Rc::new(p),
        Err(err) => {
            println!("Loading program '{}' failed.",path);
            match err {
                LoadError::ParseError(msg) =>  {
                    println!("{}",msg);
                    exit(1);
                },

                LoadError::AnalysisError(msg_vec) =>  {
                    for msg in msg_vec {
                        println!("{}",msg);
                    }
                    exit(1);
                },
            }
        },
    };

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(&program);
}
