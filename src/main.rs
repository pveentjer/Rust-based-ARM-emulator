use std::rc::Rc;

use crate::cpu::{CPU, CPUConfig, Trace};
use crate::loader::loader::load;

mod cpu;
mod loader;
mod frontend;
mod backend;
mod instructions;
mod memory_subsystem;

fn main() {
    let cpu_config = CPUConfig {
        arch_reg_count: 16,
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
            decode: false,
            issue: false,
            dispatch: false,
            execute: true,
            retire: false,
            cycle: true,
        },
        retire_n_wide: 4,
        dispatch_n_wide: 4,
        issue_n_wide: 4,
        stack_capacity: 32,
    };

    let path = "program2.asm";
    println!("Loading {}",path);
    let program = Rc::new(load(cpu_config.clone(), path));

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(&program);
}
