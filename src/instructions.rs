use std::fmt;

pub enum Opcode {
    ADD,
    SUB,
    LOAD,
    STORE,
}


pub(crate) type RegisterType = u16;
pub(crate) type MemoryType = u64;
pub(crate) type WordType = i32;

pub(crate) struct InstrQueue {
    pub(crate) head: u64,
    pub(crate) tail: u64,
}

enum OpType {
    REGISTER,
    MEMORY,
}

pub(crate) struct Instr {
    pub(crate) opcode: Opcode,
    pub(crate) sink: Vec<Operand>,
    pub(crate) source: Vec<Operand>,
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", mnemonic(&self.opcode))?;

        for source in &self.source {
            write!(f, " {}", source)?;
        }
        for sink in &self.sink {
            write!(f, " {}", sink)?;
        }

        Ok(())
    }
}


pub(crate) struct Operand {
    pub(crate) op_type: OpType,
    pub(crate) union: OpUnion,
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.union {
            OpUnion::Register(reg) => write!(f, "R{}", reg),
            OpUnion::Memory(mem) => write!(f, "[{}]", mem),
            OpUnion::Code(code) => write!(f, "[{}]", code),
            OpUnion::Constant(val) => write!(f, "{}", val),
        }
    }
}

pub(crate) enum OpUnion {
    Register(RegisterType),
    Memory(MemoryType),
    Code(MemoryType),
    Constant(WordType),
}

pub(crate) fn mnemonic(opcode: &Opcode) -> &'static str {
    match opcode {
        Opcode::ADD => "ADD",
        Opcode::SUB => "SUB",
        Opcode::LOAD => "LOAD",
        Opcode::STORE => "STORE",
    }
}



pub(crate) fn create_ADD(src_1: RegisterType, src_2: RegisterType, sink: RegisterType) -> Instr {
    Instr {
        opcode: Opcode::ADD,
        source: vec![Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_1) },
                     Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_2) }],
        sink: vec![Operand { op_type: OpType::REGISTER, union: OpUnion::Register(sink) }],
    }
}

pub(crate) fn create_SUB(src_1: RegisterType, src_2: RegisterType, sink: RegisterType) -> Instr {
    Instr {
        opcode: Opcode::SUB,
        source: vec![Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_1) },
                     Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_2) }],
        sink: vec![Operand { op_type: OpType::REGISTER, union: OpUnion::Register(sink) }],
    }
}

pub(crate) fn create_LOAD(src: MemoryType, sink: RegisterType) -> Instr {
    Instr {
        opcode: Opcode::LOAD,
        source: vec![Operand { op_type: OpType::MEMORY, union: OpUnion::Memory(src) }],
        sink: vec![Operand { op_type: OpType::REGISTER, union: OpUnion::Register(sink) }],
    }
}

pub(crate) fn create_STORE(src: RegisterType, sink: MemoryType) -> Instr {
    Instr {
        opcode: Opcode::STORE,
        source: vec![Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src) }],
        sink: vec![Operand { op_type: OpType::MEMORY, union: OpUnion::Memory(sink) }],
    }
}




