mod instructions;

use instructions::Opcode;
use instructions::Instr;
use crate::instructions::create_instr;

struct Program {
    code: Vec<Instr>,
}


fn main() {
    let mut code = Vec::<Instr>::new();
    code.push(create_instr(Opcode::ADD));
    code.push(create_instr(Opcode::STORE));
    code.push(create_instr(Opcode::LOAD));

    let program = Program { code };

    for instruction in &program.code {
        match instruction.opcode {
            Opcode::ADD => println!("ADD"),
            Opcode::SUB => println!("SUB"),
            Opcode::LOAD => println!("LOAD"),
            Opcode::STORE => println!("STORE"),
            // Handle other opcodes if needed
        }
    }
}
