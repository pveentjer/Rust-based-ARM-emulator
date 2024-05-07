mod instructions;
mod cpu;
mod loader;
mod frontend;
mod backend;

use crate::cpu::{CPU, CPUConfig};
use crate::loader::load;


fn main() {
    let program = load("bla.asm");

    let cpu_config = CPUConfig {
        arch_reg_count: 16,
        phys_reg_count: 64,
        frontend_n_wide: 1,
        instr_queue_capacity: 8,
        frequency_hz: 1,
        rs_count: 16,
    };

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(program);
}
