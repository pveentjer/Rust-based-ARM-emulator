use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use crate::cpu::{CPSR, SP};
use crate::cpu::FP;
use crate::cpu::LR;
use crate::cpu::PC;

pub type RegisterType = u16;
pub type DWordType = u64;

pub struct RegisterTypeDisplay {
    pub register: RegisterType,
}

impl Display for RegisterTypeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.register as u16 {
            FP => write!(f, "FP"),
            LR => write!(f, "LR"),
            SP => write!(f, "SP"),
            PC => write!(f, "PC"),
            CPSR => write!(f, "CPSR"),
            _ => write!(f, "R{}", self.register),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Opcode {
    ADD,
    SUB,
    RSB,
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
    RET,
    CBZ,
    CBNZ,
    // Acts like a poison pill. It isn't a public instruction.
    EXIT,
    NEG,
    AND,
    ORR,
    EOR,
    MVN,
    CMP,
    TST,
    TEQ,
    BEQ,
    BNE,
    BLE,
    BLT,
    BGE,
    BGT,
    DSB,
}

pub(crate) fn mnemonic(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::ADD => "ADD",
        Opcode::SUB => "SUB",
        Opcode::RSB => "RSB",
        Opcode::MUL => "MUL",
        Opcode::SDIV => "SDIV",
        Opcode::NEG => "NEG",
        Opcode::ADR => "ADR",
        Opcode::LDR => "LDR",
        Opcode::STR => "STR",
        Opcode::NOP => "NOP",
        Opcode::PRINTR => "PRINTR",
        Opcode::MOV => "MOV",
        Opcode::B => "B",
        Opcode::RET => "RET",
        Opcode::BX => "BX",
        Opcode::BL => "BL",
        Opcode::CBZ => "CBZ",
        Opcode::CBNZ => "CBNZ",
        Opcode::AND => "AND",
        Opcode::ORR => "ORR",
        Opcode::EOR => "EOR",
        Opcode::MVN => "MVN",
        Opcode::EXIT => "EXIT",
        Opcode::CMP => "CMP",
        Opcode::BEQ => "BEQ",
        Opcode::BNE => "BNE",
        Opcode::BLE => "BLE",
        Opcode::BLT => "BLT",
        Opcode::BGE => "BGE",
        Opcode::BGT => "BGT",
        Opcode::DSB => "DSB",
        Opcode::TST => "TST",
        Opcode::TEQ => "TEQ",
    }
}

pub(crate) fn get_opcode(mnemonic: &str) -> Option<Opcode> {
    let string = mnemonic.to_uppercase();
    let mnemonic_uppercased = string.as_str();

    match mnemonic_uppercased {
        "ADD" => Some(Opcode::ADD),
        "SUB" => Some(Opcode::SUB),
        "RSB" => Some(Opcode::RSB),
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
        "RET" => Some(Opcode::RET),
        "BX" => Some(Opcode::BX),
        "CBZ" => Some(Opcode::CBZ),
        "CBNZ" => Some(Opcode::CBNZ),
        "AND" => Some(Opcode::AND),
        "ORR" => Some(Opcode::ORR),
        "EOR" => Some(Opcode::EOR),
        "MVN" => Some(Opcode::MVN),
        "BL" => Some(Opcode::BL),
        "EXIT" => Some(Opcode::EXIT),
        "CMP" => Some(Opcode::CMP),
        "BEQ" => Some(Opcode::BEQ),
        "BNE" => Some(Opcode::BNE),
        "BLE" => Some(Opcode::BLE),
        "BLT" => Some(Opcode::BLT),
        "BGE" => Some(Opcode::BGE),
        "BGT" => Some(Opcode::BGT),
        "DSB" => Some(Opcode::DSB),
        "TST" => Some(Opcode::TST),
        "TEQ" => Some(Opcode::TEQ),
        _ => None,
    }
}

pub(crate) const NOP: Instr = Instr::Synchronization(
    Synchronization {
        opcode: Opcode::NOP,
        loc: None,
    }
);

pub(crate) const EXIT: Instr = Instr::Synchronization(
    Synchronization {
        opcode: Opcode::EXIT,
        loc: None,
    }
);

#[derive(Clone, Copy, Debug)]
pub enum Operand2 {
    Immediate {
        value: DWordType,
    },
    Register {
        reg_id: RegisterType,
    },
    Unused(),
}

impl Display for Operand2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Operand2::Immediate { value } => write!(f, "{}", *value),
            Operand2::Register { reg_id } => write!(f, "{}", RegisterTypeDisplay { register: *reg_id }),
            Operand2::Unused() => write!(f, "Unused"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConditionCode {
    EQ, // Equal
    NE, // Not Equal
    CS, // Carry Set
    CC, // Carry Clear
    MI, // Minus/Negative
    PL, // Plus/Positive or Zero
    VS, // Overflow
    VC, // No Overflow
    HI, // Unsigned Higher
    LS, // Unsigned Lower or Same
    GE, // Signed Greater Than or Equal
    LT, // Signed Less Than
    GT, // Signed Greater Than
    LE, // Signed Less Than or Equal
    AL, // Always (unconditional)
}

#[derive(Clone, Copy, Debug)]
pub struct DataProcessing {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub loc: SourceLocation,
    // First operand register.
    pub rn: Option<RegisterType>,
    // Destination register
    pub rd: RegisterType,
    // Second operand, which can be an immediate value or a shifted register.
    pub operand2: Operand2,
    // If the destination register should be read before it is written to
    pub rd_read: bool,
}

impl Display for DataProcessing {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.opcode {
            Opcode::SUB |
            Opcode::MUL |
            Opcode::SDIV |
            Opcode::AND |
            Opcode::ORR |
            Opcode::EOR |
            Opcode::RSB |
            Opcode::ADD => write!(f, "{:?} {}, {}, {}",
                                  self.opcode,
                                  RegisterTypeDisplay { register: self.rd },
                                  RegisterTypeDisplay { register: self.rn.unwrap() },
                                  self.operand2),
            Opcode::NEG|
            Opcode::MOV => write!(f, "{:?} {}, {}",
                                  self.opcode, RegisterTypeDisplay { register: self.rd }, self.operand2),
            Opcode::MVN => write!(f, "{:?} {}, {}",
                                  self.opcode, RegisterTypeDisplay { register: self.rd }, RegisterTypeDisplay{register:self.rn.unwrap()}),
            Opcode::TEQ |
            Opcode::TST |
            Opcode::CMP => write!(f, "{:?} {}, {}",
                                  self.opcode,
                                  RegisterTypeDisplay { register: self.rn.unwrap() },
                                  self.operand2),
            _ => unreachable!("Unknown opcode {:?}", self.opcode),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BranchTarget {
    Immediate {
        offset: u32,
    },
    Register {
        register: RegisterType,
    },
}

impl Display for BranchTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BranchTarget::Immediate { offset } => write!(f, "{}", offset),
            BranchTarget::Register { register } => write!(f, "{}", RegisterTypeDisplay { register: *register }),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Branch {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub loc: SourceLocation,
    pub link_bit: bool,
    pub target: BranchTarget,
    // the register to test against.
    pub rt: Option<RegisterType>,
}

impl Display for Branch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.opcode {
            Opcode::RET |
            Opcode::B |
            Opcode::BX |
            Opcode::BL => write!(f, "{:?} {}", self.opcode, self.target),
            Opcode::CBZ |
            Opcode::CBNZ => write!(f, "{:?} {}, {}", self.opcode, self.rt.unwrap(), self.target),
            Opcode::BEQ |
            Opcode::BNE |
            Opcode::BLT |
            Opcode::BLE |
            Opcode::BGT |
            Opcode::BGE => write!(f, "{:?} {}", self.opcode, self.target),
            _ => unreachable!("Unknown opcode {:?}", self.opcode),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LoadStore {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub loc: SourceLocation,
    pub rn: RegisterType,
    pub rd: RegisterType,
    pub offset: u16,
}

impl Display for LoadStore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.opcode {
            Opcode::LDR => write!(f, "LDR {}, [{}]", RegisterTypeDisplay { register: self.rd }, RegisterTypeDisplay { register: self.rn }),
            Opcode::STR => write!(f, "STR {}, [{}]", RegisterTypeDisplay { register: self.rd }, RegisterTypeDisplay { register: self.rn }),
            _ => unreachable!("Unknown opcode {:?}", self.opcode),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Synchronization {
    pub opcode: Opcode,
    pub loc: Option<SourceLocation>,
}

impl Display for Synchronization {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.opcode)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Printr {
    pub loc: Option<SourceLocation>,
    pub rn: RegisterType,
}

impl Display for Printr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PRINTR {}", RegisterTypeDisplay { register: self.rn })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Instr {
    DataProcessing(DataProcessing),
    Branch(Branch),
    LoadStore(LoadStore),
    Synchronization(Synchronization),
    Printr(Printr),
}

impl Display for Instr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Instr::DataProcessing(dp) => Display::fmt(dp, f),
            Instr::Branch(branch) => Display::fmt(branch, f),
            Instr::LoadStore(load_store) => Display::fmt(load_store, f),
            Instr::Synchronization(synchronization) => Display::fmt(synchronization, f),
            Instr::Printr(printr) => Display::fmt(printr, f),
        }
    }
}

pub(crate) struct InstrQueueSlot {
    pub(crate) instr: Rc<Instr>,
    // The pc of the current instr.
    pub(crate) pc: usize,
    pub(crate) branch_target_predicted: usize,
}

// The InstrQueue sits between frontend and backend
pub(crate) struct InstrQueue {
    pub(crate) capacity: u16,
    pub(crate) head: u64,
    pub(crate) tail: u64,
    pub(crate) slots: Vec<InstrQueueSlot>,
}

impl InstrQueue {
    pub fn new(capacity: u16) -> Self {
        let mut slots = Vec::with_capacity(capacity as usize);

        for _ in 0..capacity {
            slots.push(InstrQueueSlot { pc: 0, branch_target_predicted: 0, instr: Rc::new(NOP) });
        }

        InstrQueue {
            capacity,
            head: 0,
            tail: 0,
            slots,
        }
    }

    pub fn head_index(&self) -> usize {
        return (self.head % self.capacity as u64) as usize;
    }

    pub fn tail_index(&self) -> usize {
        return (self.tail % self.capacity as u64) as usize;
    }

    pub fn get_mut(&mut self, index: usize) -> &mut InstrQueueSlot {
        return self.slots.get_mut(index).unwrap();
    }

    pub fn tail_bump(&mut self) {
        self.tail += 1;
    }

    pub fn head_bump(&mut self) {
        self.head += 1;
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

    pub fn flush(&mut self) {
        self.head = 0;
        self.tail = 0;
    }
}

// True if the instruction is a control instruction; so a partly serializing instruction (no other instructions)
// A control instruction gets issued into the rob, but it will prevent the next instruction to be issued, so
// That the branch condition can be determined.
pub(crate) const INSTR_FLAG_IS_BRANCH: u8 = 0;
pub(crate) const INSTR_FLAG_SB_SYNC: u8 = 1;
pub(crate) const INSTR_FLAG_ROB_SYNC: u8 = 2;

pub struct Data {
    pub value: DWordType,
    pub offset: u64,
}

pub struct Program {
    pub data_items: HashMap::<String, Rc<Data>>,
    pub code: Vec<Rc<Instr>>,
    pub entry_point: usize,
}

impl Program {
    pub fn get_instr(&self, pos: usize) -> Rc<Instr> {
        Rc::clone(&self.code[pos])
    }
}

