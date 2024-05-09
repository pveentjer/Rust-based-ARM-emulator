mod instructions;
mod cpu;
mod loader;
mod frontend;
mod backend;
mod memory_subsystem;

use crate::cpu::{CPU, CPUConfig};
use crate::loader::load;


fn main() {
    let program = load("bla.asm");

    let cpu_config = CPUConfig {
        arch_reg_count: 16,
        phys_reg_count: 64,
        frontend_n_wide: 4,
        instr_queue_capacity: 8,
        frequency_hz: 1,
        rs_count: 16,
        memory_size: 32,
        sb_capacity: 16,
        lfb_count: 8,
        rob_capacity: 8,
        eu_count: 8,
        trace: true,
        retire_n_wide: 4,
    };

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(program);
}
