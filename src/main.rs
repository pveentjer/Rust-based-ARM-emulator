use std::process::exit;
use std::rc::Rc;
use lalrpop_util::lalrpop_mod;

use crate::cpu::{CPU, CPUConfig, Trace};
use crate::loader::loader::{load, LoadError};

mod cpu;
mod loader;
mod frontend;
mod backend;
mod instructions;
mod memory_subsystem;


lalrpop_mod!(pub assembly, "/loader/assembly.rs");

fn main() {
    let cpu_config = CPUConfig {
        phys_reg_count: 64,
        frontend_n_wide: 4,
        instr_queue_capacity: 8,
        frequency_hz: 4,
        rs_count: 16,
        memory_size: 128,
        sb_capacity: 16,
        lfb_count: 8,
        rob_capacity: 32,
        eu_count: 16,
        trace: Trace {
            decode: false,
            issue: false,
            dispatch: false,
            execute: false,
            retire: false,
            cycle: false,
        },
        retire_n_wide: 4,
        dispatch_n_wide: 4,
        issue_n_wide: 4,
        stack_capacity: 32,
    };

    let path = "foo.asm";
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
