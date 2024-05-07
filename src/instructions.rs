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

// The InstrQueue sits between frontend and backend
// The 'a lifetime specifier tells that the instructions need to live as least as long
// as the instructoin queue.
pub(crate) struct InstrQueue<'a> {
    capacity: u16,
    head: u16,
    tail: u16,
    instructions: Vec<Option<&'a Instr>>,
}

impl<'a> InstrQueue<'a> {
    pub fn new(capacity: u16) -> Self {
        InstrQueue {
            capacity,
            head: 0,
            tail: 0,
            instructions: vec![None; capacity as usize],
        }
    }

    pub fn size(&self) -> u16 {
        self.tail.wrapping_sub(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        self.size() == self.capacity
    }

    pub fn enqueue(&mut self, instr: &'a Instr) {
        if !self.is_full() {
            let index = (self.tail % self.capacity) as usize;
            self.instructions[index] = Some(instr);
            self.tail = self.tail.wrapping_add(1);
        } else {
            // Handle queue full scenario
            // For now, just print an error message
            println!("Queue is full, cannot enqueue.");
        }
    }

    pub fn dequeue(&mut self) -> Option<&'a Instr> {
        if !self.is_empty() {
            let index = (self.head % self.capacity) as usize;
            let instr = self.instructions[index].take();
            self.head = self.head.wrapping_add(1);
            instr
        } else {
            None
        }
    }
}


pub(crate) enum OpType {
    REGISTER,
    MEMORY,
    UNUSED,
}

const MAX_SINK_COUNT: u8 = 1;
const MAX_SOURCE_COUNT: u8 = 2;

pub(crate) struct Instr {
    pub(crate) opcode: Opcode,
    pub(crate) sink_cnt: u8,
    pub(crate) sink: [Operand; MAX_SINK_COUNT as usize],
    pub(crate) source_cnt: u8,
    pub(crate) source: [Operand; MAX_SOURCE_COUNT as usize],
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", mnemonic(&self.opcode))?;

        for k in 0..self.source_cnt {
            write!(f, " {}", self.source[k as usize])?;
        }

        for k in 0..self.sink_cnt {
            write!(f, " {}", self.sink[k as usize])?;
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
            OpUnion::Unused => write!(f, "unused"),
        }
    }
}

pub(crate) enum OpUnion {
    Register(RegisterType),
    Memory(MemoryType),
    Code(MemoryType),
    Constant(WordType),
    Unused,
}

pub(crate) fn mnemonic(opcode: &Opcode) -> &'static str {
    match opcode {
        Opcode::ADD => "ADD",
        Opcode::SUB => "SUB",
        Opcode::LOAD => "LOAD",
        Opcode::STORE => "STORE",
    }
}

pub(crate) struct Program {
    pub(crate) code: Vec<Instr>,
}

pub(crate) fn create_ADD(src_1: RegisterType, src_2: RegisterType, sink: RegisterType) -> Instr {
    Instr {
        opcode: Opcode::ADD,
        source_cnt: 2,
        source: [
            Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_1) },
            Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_2) }],
        sink_cnt: 1,
        sink: [Operand { op_type: OpType::REGISTER, union: OpUnion::Register(sink) }],
    }
}

pub(crate) fn create_SUB(src_1: RegisterType, src_2: RegisterType, sink: RegisterType) -> Instr {
    Instr {
        opcode: Opcode::SUB,
        source_cnt: 2,
        source: [
            Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_1) },
            Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src_2) }],
        sink_cnt: 1,
        sink: [Operand { op_type: OpType::REGISTER, union: OpUnion::Register(sink) }],
    }
}

pub(crate) fn create_LOAD(addr: MemoryType, sink: RegisterType) -> Instr {
    Instr {
        opcode: Opcode::LOAD,
        source_cnt: 1,
        source: [
            Operand { op_type: OpType::MEMORY, union: OpUnion::Memory(addr) },
            Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
        ],
        sink_cnt: 1,
        sink: [Operand { op_type: OpType::REGISTER, union: OpUnion::Register(sink) }],
    }
}

pub(crate) fn create_STORE(src: RegisterType, addr: MemoryType) -> Instr {
    Instr {
        opcode: Opcode::STORE,
        source_cnt: 1,
        source: [
            Operand { op_type: OpType::REGISTER, union: OpUnion::Register(src) },
            Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
        ],
        sink_cnt: 1,
        sink: [Operand { op_type: OpType::MEMORY, union: OpUnion::Memory(addr) }],
    }
}
