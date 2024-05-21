use std::fs;
use lalrpop_util::lalrpop_mod;

use crate::cpu::{CPU, CPUConfig, Trace};

mod cpu;
mod loader;
mod frontend;
mod backend;
mod instructions;
mod memory_subsystem;
mod ast;


lalrpop_mod!(pub assembly);

fn main() {
    let cpu_config = CPUConfig {
        phys_reg_count: 64,
        frontend_n_wide: 4,
        instr_queue_capacity: 8,
        frequency_hz: 4,
        rs_count: 16,
        memory_size: 32,
        sb_capacity: 16,
        lfb_count: 8,
        rob_capacity: 32,
        eu_count: 16,
        trace: Trace {
            decode: true,
            issue: true,
            dispatch: true,
            execute: true,
            retire: true,
            cycle: true,
        },
        retire_n_wide: 4,
        dispatch_n_wide: 4,
        issue_n_wide: 4,
        stack_capacity: 32,
    };

    let path = "foo.asm";


     let mut input = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            panic!("Error reading file: {}", err);
        }
    };

    let parse_result = assembly::AssemblyParser::new()
        .parse(input.as_str());

    match parse_result {
        Ok(_)=>{
            println!("Parse success");
        }
        Err(e)=>{
            panic!("{}",e);
        }
    }
}
