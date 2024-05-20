use std::collections::HashMap;
use std::fmt;

use std::rc::Rc;
use crate::cpu::{GENERAL_ARG_REG_CNT, SP};
use crate::cpu::LR;
use crate::cpu::PC;
use crate::cpu::FP;


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Opcode {
    ADD,
    SUB,
    MUL,
    SDIV,
    ADR,
    LDR,
    STR,
    NOP,
    PRINTR,
    MOV,
    B,
    BX,
    BL,
    CBZ,
    CBNZ,
    // remove
    EXIT,
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
}

pub(crate) fn mnemonic(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::ADD => "ADD",
        Opcode::SUB => "SUB",
        Opcode::MUL => "MUL",
        Opcode::SDIV => "SDIV",
        Opcode::NEG => "NEG",
        Opcode::ADR => "ADR",
        Opcode::LDR => "LDR",
        Opcode::STR => "STR",
        Opcode::NOP => "NOP",
        Opcode::PRINTR => "PRINTR",
        Opcode::MOV => "PRINTR",
        Opcode::B => "B",
        Opcode::BX => "BX",
        Opcode::BL => "BL",
        Opcode::CBZ => "CBZ",
        Opcode::CBNZ => "CBNZ",
        Opcode::PUSH => "PUSH",
        Opcode::POP => "POP",
        Opcode::AND => "AND",
        Opcode::ORR => "ORR",
        Opcode::EOR => "EOR",
        Opcode::NOT => "NOT",
        Opcode::EXIT => "EXIT",
    }
}

pub(crate) fn get_opcode(mnemonic: &str) -> Option<Opcode> {
    let string = mnemonic.to_uppercase();
    let mnemonic_uppercased = string.as_str();

    match mnemonic_uppercased {
        "ADD" => Some(Opcode::ADD),
        "SUB" => Some(Opcode::SUB),
        "MUL" => Some(Opcode::MUL),
        "SDIV" => Some(Opcode::SDIV),
        "NEG" => Some(Opcode::NEG),
        "ADR" => Some(Opcode::ADR),
        "LDR" => Some(Opcode::LDR),
        "STR" => Some(Opcode::STR),
        "NOP" => Some(Opcode::NOP),
        "PRINTR" => Some(Opcode::PRINTR),
        "MOV" => Some(Opcode::MOV),
        "B" => Some(Opcode::B),
        "BX" => Some(Opcode::BX),
        "CBZ" => Some(Opcode::CBZ),
        "CBNZ" => Some(Opcode::CBZ),
        "PUSH" => Some(Opcode::PUSH),
        "POP" => Some(Opcode::POP),
        "AND" => Some(Opcode::AND),
        "ORR" => Some(Opcode::ORR),
        "EOR" => Some(Opcode::EOR),
        "NOT" => Some(Opcode::NOT),
        "BL" => Some(Opcode::BL),
        "EXIT" => Some(Opcode::EXIT),
        _ => None,
    }
}

pub(crate) fn get_register(name:&str)->Option<u16>{
    let name_uppercased = name.to_uppercase();

    match name_uppercased.as_str() {
        "SP" => Some(SP),
        "LR" => Some(LR),
        "PC" => Some(PC),
        "FP" => Some(FP),
        _ => {
            let reg_name = &name_uppercased[1..];
            let reg: u16 = reg_name.parse().unwrap();

            if reg >= GENERAL_ARG_REG_CNT {
                return None;
            }
            Some(reg)
        }
    }
}

pub(crate) const NOP: Instr = create_NOP(-1);

pub(crate) type RegisterType = u16;
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
    // True if the instruction is a control instruction; so a partly serializing instruction (no other instructions)
    pub(crate) is_control: bool,
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
            Opcode::ADR => write!(f, "{},{}", self.sink[0], self.source[0])?,
            Opcode::PRINTR => write!(f, "{}", self.source[0])?,
            Opcode::B |
            Opcode::BX |
            Opcode::BL => write!(f, "{}", self.source[0])?,
            Opcode::CBZ |
            Opcode::CBNZ => write!(f, "{},{}", self.source[0], self.source[1])?,
            Opcode::PUSH => {}
            Opcode::POP => {}
            Opcode::NEG => {}
            Opcode::NOT => {}
            Opcode::EXIT => {}
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
    Memory(WordType),

    Code(WordType),

    Unused,
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Register(reg) => {
                match *reg as u16 {
                    FP => write!(f, "FP"),
                    LR => write!(f, "LR"),
                    SP => write!(f, "SP"),
                    PC => write!(f, "PC"),
                    _ => write!(f, "R{}", reg),
                }
            }  // Add a comma here
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

    pub(crate) fn get_code_address(&self) -> WordType {
        match self {
            Operand::Code(constant) => *constant,
            _ => panic!("Operand is not a Code but of type {:?}", self),
        }
    }

    pub(crate) fn get_memory_addr(&self) -> WordType {
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
    pub(crate) entry_point: usize,
}

impl Program {
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
        is_control: false,
    }
}
