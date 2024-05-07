use crate::Opcode::LOAD;

enum Opcode {
    ADD,
    SUB,
    LOAD,
    STORE,
}

struct Instr {
    opcode: Opcode,
    sink: Vec<Operand>,
    source: Vec<Operand>,
}

struct Program {
    code: Vec<Instr>,
}

struct InstrQueue {
    head: u64,
    tail: u64,
}

enum OperandType {
    REGISTER,
    MEMORY,
}

struct Operand {
    opType: OperandType,
    union: OperandUnion,
}


enum OperandUnion {
    Register(u16),
    Memory(u64),
    Code(u64),
    Constant(i32),
}

fn create_instr(opcode: Opcode) -> Instr {
    match opcode {
        Opcode::ADD => Instr { opcode: Opcode::ADD, source: vec![], sink: vec![] },
        Opcode::SUB => Instr { opcode: Opcode::SUB, source: vec![], sink: vec![] },
        Opcode::LOAD => Instr { opcode: Opcode::LOAD, source: vec![], sink: vec![] },
        Opcode::STORE => Instr { opcode: Opcode::STORE, source: vec![], sink: vec![] },
        // Handle other opcodes if needed
    }
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
