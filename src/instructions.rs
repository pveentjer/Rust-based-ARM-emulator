
pub enum Opcode {
    ADD,
    SUB,
    LOAD,
    STORE,
}


pub(crate) struct InstrQueue {
    pub(crate) head: u64,
    pub(crate) tail: u64,
}

enum OperandType {
    REGISTER,
    MEMORY,
}

pub(crate) struct Instr {
    pub(crate) opcode: Opcode,
    pub(crate) sink: Vec<Operand>,
    pub(crate) source: Vec<Operand>,
}


pub(crate) struct Operand {
    pub(crate) opType: OperandType,
    pub(crate) union: OperandUnion,
}


pub(crate) enum OperandUnion {
    Register(u16),
    Memory(u64),
    Code(u64),
    Constant(i32),
}

pub(crate) fn create_instr(opcode: Opcode) -> Instr {
    match opcode {
        Opcode::ADD => Instr { opcode: Opcode::ADD, source: vec![], sink: vec![] },
        Opcode::SUB => Instr { opcode: Opcode::SUB, source: vec![], sink: vec![] },
        Opcode::LOAD => Instr { opcode: Opcode::LOAD, source: vec![], sink: vec![] },
        Opcode::STORE => Instr { opcode: Opcode::STORE, source: vec![], sink: vec![] },
        // Handle other opcodes if needed
    }
}