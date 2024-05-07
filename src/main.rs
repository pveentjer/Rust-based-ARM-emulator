mod instructions;
mod cpu;

use instructions::Opcode;
use instructions::Instr;
use crate::cpu::{CPU, CPUConfig};
use crate::instructions::{create_ADD, create_LOAD, create_STORE};

struct Program {
    code: Vec<Instr>,
}


fn main() {
    let mut code = Vec::<Instr>::new();
    code.push(create_LOAD(0, 0));
    code.push(create_LOAD(1, 1));
    code.push(create_ADD(0, 1, 2));
    code.push(create_STORE(2, 2));

    let program = Program { code };

    for instr in &program.code {
       println!("{}", instr);
    }

    let mut cpu_config = CPUConfig{
        arch_reg_count: 16,
        phys_reg_count: 64,
    };
    let mut cpu = CPU{};
}
