use std::collections::HashMap;
use std::fmt;
use std::ptr::write;
use std::rc::Rc;
use crate::cpu::ARCH_REG_SP;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Opcode {
    ADD,
    SUB,
    MUL,
    SDIV,
    LDR,
    STR,
    NOP,
    PRINTR,
    MOV,
    B,
    // remove
    JNZ,
    // remove
    JZ,
    // remove
    PUSH,
    // remove
    POP,
    NEG,
    AND,
    ORR,
    EOR,
    // remove
    NOT,
    BL,
    // called differently
    RET,
    // remove
    EXIT,
}

pub(crate) fn is_control(opcode: Opcode) -> bool {
    return match opcode {
        Opcode::JNZ => true,
        Opcode::BL => true,
        Opcode::RET => true,
        Opcode::B => true,
        _ => false,
    };
}

pub(crate) fn mnemonic(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::ADD => "ADD",
        Opcode::SUB => "SUB",
        Opcode::MUL => "MUL",
        Opcode::SDIV => "SDIV",
        Opcode::NEG => "NEG",
        Opcode::LDR => "LDR",
        Opcode::STR => "STR",
        Opcode::NOP => "NOP",
        Opcode::PRINTR => "PRINTR",
        Opcode::MOV => "PRINTR",
        Opcode::B => "B",
        Opcode::JNZ => "JNZ",
        Opcode::JZ => "JZ",
        Opcode::PUSH => "PUSH",
        Opcode::POP => "POP",
        Opcode::AND => "AND",
        Opcode::ORR => "ORR",
        Opcode::EOR => "EOR",
        Opcode::NOT => "NOT",
        Opcode::BL => "BL",
        Opcode::RET => "RET",
        Opcode::EXIT => "EXIT",
    }
}

pub(crate) fn get_opcode(name: &str) -> Option<Opcode> {
    match name {
        "ADD" => Some(Opcode::ADD),
        "SUB" => Some(Opcode::SUB),
        "MUL" => Some(Opcode::MUL),
        "SDIV" => Some(Opcode::SDIV),
        "NEG" => Some(Opcode::NEG),
        "LOAD" => Some(Opcode::LDR),
        "STORE" => Some(Opcode::STR),
        "NOP" => Some(Opcode::NOP),
        "PRINTR" => Some(Opcode::PRINTR),
        "MOV" => Some(Opcode::MOV),
        "B" => Some(Opcode::B),
        "JNZ" => Some(Opcode::JNZ),
        "JZ" => Some(Opcode::JZ),
        "PUSH" => Some(Opcode::PUSH),
        "POP" => Some(Opcode::POP),
        "AND" => Some(Opcode::AND),
        "ORR" => Some(Opcode::ORR),
        "EOR" => Some(Opcode::EOR),
        "NOT" => Some(Opcode::NOT),
        "BL" => Some(Opcode::BL),
        "RET" => Some(Opcode::RET),
        "EXIT" => Some(Opcode::RET),
        _ => None,
    }
}

pub(crate) const NOP: Instr = create_NOP(-1);

pub(crate) type RegisterType = u16;
pub(crate) type MemoryAddressType = u64;
pub(crate) type CodeAddressType = u64;
pub(crate) type WordType = i64;

// The InstrQueue sits between frontend and backend
// The 'a lifetime specifier tells that the instructions need to live as least as long
// as the instruction queue.
pub(crate) struct InstrQueue {
    capacity: u16,
    head: u64,
    tail: u64,
    instructions: Vec<Rc<Instr>>,
}

impl InstrQueue {
    pub fn new(capacity: u16) -> Self {
        let mut instructions = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            instructions.push(Rc::new(NOP));
        }

        InstrQueue {
            capacity,
            head: 0,
            tail: 0,
            instructions,
        }
    }

    pub fn size(&self) -> u16 {
        (self.tail - self.head) as u16
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        self.size() == self.capacity
    }

    pub fn enqueue(&mut self, instr: Rc<Instr>) {
        assert!(!self.is_full(), "Can't enqueue when InstrQueue is empty.");

        let index = (self.tail % self.capacity as u64) as usize;
        self.instructions[index] = instr;
        self.tail += 1;
    }

    pub fn dequeue(&mut self) {
        assert!(!self.is_empty(), "Can't dequeue when InstrQueue is empty.");
        self.head += 1;
    }

    pub fn peek(&self) -> Rc<Instr> {
        assert!(!self.is_empty(), "Can't peek when InstrQueue is empty.");

        let index = (self.head % self.capacity as u64) as usize;
        return Rc::clone(&self.instructions[index]);
    }
}

// The maximum number of source (input) operands for an instruction.
pub(crate) const MAX_SOURCE_COUNT: u8 = 3;
pub(crate) const MAX_SINK_COUNT: u8 = 2;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Instr {
    pub(crate) cycles: u8,
    pub(crate) opcode: Opcode,
    pub(crate) source_cnt: u8,
    pub(crate) source: [Operand; MAX_SOURCE_COUNT as usize],
    pub(crate) sink_cnt: u8,
    pub(crate) sink: [Operand; MAX_SINK_COUNT as usize],
    pub(crate) line: i32,
    pub(crate) mem_stores: u8,
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", mnemonic(self.opcode))?;

        match self.opcode {
            Opcode::ADD |
            Opcode::SUB |
            Opcode::MUL |
            Opcode::SDIV |
            Opcode::AND |
            Opcode::ORR |
            Opcode::EOR => write!(f, "{},{},{}", self.sink[0], self.source[0], self.source[1])?,
            Opcode::LDR => write!(f, "{},{}", self.sink[0], self.source[0])?,
            Opcode::STR => write!(f, "{},{}", self.source[0], self.sink[0])?,
            Opcode::MOV => write!(f, "{},{}", self.sink[0], self.source[0])?,
            Opcode::NOP => {}
            Opcode::PRINTR => write!(f, "{}", self.source[0])?,
            Opcode::B => write!(f, "{}", self.source[0])?,
            Opcode::JNZ => {}
            Opcode::JZ => {}
            Opcode::PUSH => {}
            Opcode::POP => {}
            Opcode::NEG => {}
            Opcode::NOT => {}
            Opcode::BL => {}
            Opcode::EXIT => {}
            Opcode::RET => {}
        }

        if self.line > 0 {
            write!(f, " ; line={}", self.line)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operand {
    Register(RegisterType),
    // The operand is directly specified in the instruction itself.
    Immediate(WordType),
    // todo: rename to direct?
    Memory(MemoryAddressType),

    Code(CodeAddressType),

    Unused,
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Register(reg) => {
                if *reg == ARCH_REG_SP as RegisterType {
                    write!(f, "SP")
                } else {
                    write!(f, "R{}", reg)
                }
            }
            Operand::Immediate(val) => write!(f, "{}", val),
            Operand::Memory(addr) => write!(f, "[{}]", addr),
            Operand::Code(addr) => write!(f, "[{}]", addr),
            Operand::Unused => write!(f, "Unused"),
        }
    }
}

//Indexed(u8, i16),   // Indexed addressing mode (base register and offset).
//Indirect(u8),

impl Operand {
    pub(crate) fn get_register(&self) -> RegisterType {
        match *self {
            Operand::Register(reg) => reg,
            _ => panic!("Operation is not a Register but of type {:?}", self),
        }
    }

    pub(crate) fn get_constant(&self) -> WordType {
        match self {
            Operand::Immediate(constant) => *constant,
            _ => panic!("Operand is not a Constant but of type {:?}", self),
        }
    }

    pub(crate) fn get_code_address(&self) -> CodeAddressType {
        match self {
            Operand::Code(constant) => *constant,
            _ => panic!("Operand is not a Code but of type {:?}", self),
        }
    }

    pub(crate) fn get_memory_addr(&self) -> MemoryAddressType {
        match self {
            Operand::Memory(addr) => *addr,
            _ => panic!("Operand is not a Memory but of type {:?}", self),
        }
    }
}

pub(crate) struct Data {
    pub(crate) value: WordType,
    pub(crate) offset: u64,
}

pub(crate) struct Program {
    pub(crate) data_items: HashMap::<String, Rc<Data>>,
    pub(crate) code: Vec<Rc<Instr>>,
}

impl Program {
    pub fn new(code: Vec<Rc<Instr>>, data_items: HashMap::<String, Rc<Data>>) -> Self {
        Self { code, data_items }
    }

    pub fn get_instr(&self, pos: usize) -> Rc<Instr> {
        Rc::clone(&self.code[pos])
    }
}

pub(crate) const fn create_NOP(line: i32) -> Instr {
    Instr {
        cycles: 1,
        opcode: Opcode::NOP,
        source_cnt: 0,
        source: [Operand::Unused, Operand::Unused, Operand::Unused],
        sink_cnt: 0,
        sink: [Operand::Unused, Operand::Unused],
        line,
        mem_stores: 0,
    }
}
