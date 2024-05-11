mod instructions;
mod cpu;
mod loader;
mod frontend;
mod backend;
mod memory_subsystem;

use std::rc::Rc;
use crate::cpu::{CPU, CPUConfig};
use crate::loader::load;

fn main() {
   let cpu_config = CPUConfig {
        arch_reg_count: 16,
        phys_reg_count: 64,
        frontend_n_wide: 1,
        instr_queue_capacity: 8,
        frequency_hz: 1,
        rs_count: 16,
        memory_size: 32,
        sb_capacity: 16,
        lfb_count: 8,
        rob_capacity: 32,
        eu_count: 16,
        trace: true,
        retire_n_wide: 1,
        dispatch_n_wide: 1,
        issue_n_wide: 1
    };

    let program = Rc::new(load(&cpu_config,"program.asm",));

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(&program);
}
