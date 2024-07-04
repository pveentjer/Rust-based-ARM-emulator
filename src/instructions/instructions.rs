use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::rc::Rc;

use Operand::Memory;

use crate::cpu::{CPSR, SP};
use crate::cpu::FP;
use crate::cpu::LR;
use crate::cpu::PC;
use crate::instructions::instructions::Operand::{Code, Immediate, MemRegisterIndirect, Register, Unused};

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
        _ => None,
    }
}


pub(crate) fn create_instr(
    opcode: Opcode,
    operands: &Vec<Operand>,
    loc: SourceLocation,
) -> Result<Instr, String> {

    // let mut instr = Instr {
    //     cycles: 1,
    //     opcode,
    //     source_cnt: 0,
    //     source: [Unused, Unused, Unused],
    //     sink_cnt: 0,
    //     sink: [Unused, Unused],
    //     loc: Some(loc),
    //     mem_stores: 0,
    //     flags: 0,
    //     condition_code: ConditionCode::AL,
    // };

    let mut instr = match opcode {
        Opcode::SUB |
        Opcode::MUL |
        Opcode::SDIV |
        Opcode::AND |
        Opcode::ORR |
        Opcode::EOR |
        Opcode::RSB |
        Opcode::ADD => {
            validate_operand_count(3, operands, opcode, loc)?;

            let rd = operands[0].get_register();
            let rn = operands[1].get_register();

            // todo: ugly
            let operand2 = match operands[2] {
                Register(register) => Operand2::Register { register },
                Immediate(value) => Operand2::Immediate { value },
                _ => { panic!() }
            };

            let mut instr = Instr::DataProcessing {
                data_processing: DataProcessing {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rn,
                    rd,
                    operand2,
                }
            };
            //
            // instr.sink_cnt = 1;
            // instr.sink[0] = validate_operand(0, operands, opcode, &[Register(0)])?;
            //
            // instr.source_cnt = 2;
            // instr.source[0] = validate_operand(1, operands, opcode, &[Register(0)])?;
            // instr.source[1] = validate_operand(2, operands, opcode, &[Register(0), Immediate(0)])?;
            instr
        }
        Opcode::ADR => { panic!() }
        Opcode::STR|
        Opcode::LDR => {
            validate_operand_count(2, operands, opcode, loc)?;

            let rd = operands[0].get_register();
            let rn = operands[1].get_register();

            Instr::LoadStore {
                load_store: LoadStore {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rd: rd,
                    rn: rn,
                    offset: 0,
                }
            }
        }
        Opcode::PRINTR => {
            validate_operand_count(1, operands, opcode, loc)?;

            let rn = operands[0].get_register();

            Instr::Printr {
                printr: Printr {
                    loc: Some(loc),
                    rn,
                }
            }
        }
        Opcode::MOV => {
            validate_operand_count(2, operands, opcode, loc)?;

            let rd = operands[0].get_register();
            let rn = operands[1].get_register();

            Instr::DataProcessing {
                data_processing: DataProcessing {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rn,
                    rd,
                    operand2: Operand2::Unused(),
                }
            }
        }
        Opcode::B => {
            validate_operand_count(1, operands, opcode, loc)?;
            //
            // instr.source_cnt = 1;
            // instr.source[0] = validate_operand(0, operands, opcode, &[Code(0)])?;
            //
            // instr.sink_cnt = 0;
            // instr.set_branch();

            panic!();
        }
        Opcode::RET => {
            if operands.len() > 1 {
                return Err(format!("Operand count mismatch. {:?} expects 0 or 1 argument, but {} are provided at {}:{}",
                                   opcode, operands.len(), loc.line, loc.column));
            }

            // instr.source_cnt = 1;
            // instr.source[0] = if operands.len() == 0 {
            //     Register(LR)
            // } else {
            //     validate_operand(0, operands, opcode, &[Register(0)])?
            // };
            //
            // instr.sink_cnt = 0;
            // instr.set_branch();
            panic!();
        }
        Opcode::BX => {
            validate_operand_count(1, operands, opcode, loc)?;

            // instr.source_cnt = 1;
            // instr.source[0] = validate_operand(0, operands, opcode, &[Register(0)])?;
            //
            // instr.sink_cnt = 0;
            // instr.set_branch();

            panic!();
        }
        Opcode::BL => {
            validate_operand_count(1, operands, opcode, loc)?;

            // instr.source_cnt = 1;
            // instr.source[0] = validate_operand(0, operands, opcode, &[Code(0)])?;
            //
            // instr.sink_cnt = 1;
            // instr.sink[0] = Register(LR);
            // instr.set_branch();

            panic!();
        }
        Opcode::CBZ |
        Opcode::CBNZ => {
            validate_operand_count(2, operands, opcode, loc)?;

            // instr.source_cnt = 2;
            // instr.source[0] = validate_operand(0, operands, opcode, &[Register(0)])?;
            // instr.source[1] = validate_operand(1, operands, opcode, &[Code(0)])?;
            //
            // instr.sink_cnt = 0;
            // instr.set_branch();

            panic!();
        }
        Opcode::NOP |
        Opcode::EXIT |
        Opcode::DSB => {
            validate_operand_count(0, operands, opcode, loc)?;

            Instr::Synchronization {
                synchronization: Synchronization {
                    opcode,
                    loc: Some(loc),
                }
            }
        }
        Opcode::NEG => {
            validate_operand_count(2, operands, opcode, loc)?;

            // instr.sink_cnt = 1;
            // instr.sink[0] = validate_operand(0, operands, opcode, &[Register(0)])?;
            //
            // instr.source_cnt = 1;
            // instr.source[0] = validate_operand(1, operands, opcode, &[Register(0)])?;

            panic!();
        }
        Opcode::MVN => {
            validate_operand_count(2, operands, opcode, loc)?;

            // instr.sink_cnt = 1;
            // instr.sink[0] = validate_operand(0, operands, opcode, &[Register(0)])?;
            //
            // instr.source_cnt = 1;
            // instr.source[0] = validate_operand(1, operands, opcode, &[Immediate(0), Register(0)])?;

            panic!();
        }
        Opcode::CMP => {
            validate_operand_count(2, operands, opcode, loc)?;

            // instr.source_cnt = 3;
            // instr.source[0] = validate_operand(0, operands, opcode, &[Register(0)])?;
            // instr.source[1] = validate_operand(1, operands, opcode, &[Immediate(0), Register(0)])?;
            // instr.source[2] = Register(CPSR);
            //
            // instr.sink_cnt = 1;
            // instr.sink[0] = Register(CPSR);

            panic!();
        }
        Opcode::BEQ |
        Opcode::BNE |
        Opcode::BLT |
        Opcode::BLE |
        Opcode::BGT |
        Opcode::BGE => {
            validate_operand_count(1, operands, opcode, loc)?;

            Instr::Branch {
                branch: Branch {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    link_bit: false,
                    offset: 0,
                }
            }

            // instr.source_cnt = 2;
            // instr.source[0] = validate_operand(0, operands, opcode, &[Code(0)])?;
            // instr.source[1] = Register(CPSR);
            //
            // instr.sink_cnt = 0;
            // instr.set_branch();
        }
    };

    // todo: handling of instructions with control like modifying the IP need to be detected.
    //
    // if !instr.is_branch() && has_control_operands(&instr) {
    //     instr.set_branch();
    // }

    return Ok(instr);
}

fn validate_operand_count(expected: usize,
                          operands: &Vec<Operand>,
                          opcode: Opcode,
                          loc: SourceLocation) -> Result<(), String> {
    if operands.len() != expected {
        return Err(format!("Operand count mismatch. {:?} expects {} arguments, but {} are provided at {}:{}",
                           opcode, expected, operands.len(), loc.line, loc.column));
    }
    Ok(())
}

fn validate_operand(
    op_index: usize,
    operands: &Vec<Operand>,
    opcode: Opcode,
    acceptable_types: &[Operand],
) -> Result<Operand, String> {
    let operand = operands[op_index];

    for &typ in acceptable_types {
        if std::mem::discriminant(&operand) == std::mem::discriminant(&typ) {
            return Ok(operand);
        }
    }
    let acceptable_names: Vec<&str> = acceptable_types.iter().map(|t| t.base_name()).collect();
    let acceptable_names_str = acceptable_names.join(", ");

    Err(format!("Operand type mismatch. {:?} expects {} as argument nr {}, but {} was provided",
                opcode, acceptable_names_str, op_index + 1, operand.base_name()))
}
//
// fn has_control_operands(instr: &Instr) -> bool {
//     instr.source.iter().any(|op| is_control_operand(op)) ||
//         instr.sink.iter().any(|op| is_control_operand(op))
// }
//
// fn is_control_operand(op: &Operand) -> bool {
//     matches!(op, Register(register) if *register == PC)
// }

pub(crate) const NOP: Instr = Instr::Synchronization {
    synchronization: Synchronization {
        opcode: Opcode::NOP,
        loc: None,
    }
};

pub(crate) const EXIT: Instr = Instr::Synchronization {
    synchronization: Synchronization {
        opcode: Opcode::EXIT,
        loc: None,
    }
};

#[derive(Clone, Copy, Debug)]
pub enum Operand2 {
    Immediate {
        value: DWordType,
    },
    Register {
        register: RegisterType,
    },
    Unused(),
}

#[derive(Clone, Copy, Debug)]
pub struct DataProcessing {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub loc: SourceLocation,
    // First operand register.
    pub rn: RegisterType,
    // Destination register
    pub rd: RegisterType,
    // Second operand, which can be an immediate value or a shifted register.
    pub operand2: Operand2,
}

#[derive(Clone, Copy, Debug)]
pub struct Branch {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub loc: SourceLocation,
    pub link_bit: bool,
    pub offset: u32,
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

#[derive(Clone, Copy, Debug)]
pub struct Synchronization {
    pub opcode: Opcode,
    pub loc: Option<SourceLocation>,
}

#[derive(Clone, Copy, Debug)]
pub struct Printr {
    pub loc: Option<SourceLocation>,
    pub rn: RegisterType,
}

#[derive(Clone, Copy, Debug)]
pub enum Instr {
    DataProcessing {
        data_processing: DataProcessing,
    },

    Branch {
        branch: Branch,
    },

    LoadStore {
        load_store: LoadStore,
    },

    Synchronization {
        synchronization: Synchronization,
    },

    Printr {
        printr: Printr,
    },
}

impl Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Instr::DataProcessing { data_processing: fields } => {
                write!(f, "DataProcessing: opcode={:?}, condition={:?}, loc=({}, {}), rn={:?}, rd={:?}, operand2={:?}",
                       fields.opcode, fields.condition, fields.loc.line, fields.loc.column, fields.rn, fields.rd, fields.operand2)
            }
            Instr::Branch { branch: fields } => {
                write!(f, "Branch: opcode={:?}, condition={:?}, loc=({}, {}), link_bit={}, offset={}",
                       fields.opcode, fields.condition, fields.loc.line, fields.loc.column, fields.link_bit, fields.offset)
            }
            Instr::LoadStore { load_store: fields } => {
                write!(f, "LoadStore: opcode={:?}, condition={:?}, loc=({}, {}), rn={:?}, rt={:?}, offset={}",
                       fields.opcode, fields.condition, fields.loc.line, fields.loc.column, fields.rn, fields.rd, fields.offset)
            }
            Instr::Synchronization { synchronization: fields } => {
                write!(f, "{:?}", fields.opcode)
            }
            Instr::Printr { printr: fields } => {
                write!(f, "PRINTR {}", fields.rn)
            }
        }
    }
}


pub(crate) type RegisterType = u16;
pub(crate) type DWordType = u64;

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

// The maximum number of source (input) operands for an instruction.
pub(crate) const MAX_SOURCE_COUNT: u8 = 3;
pub(crate) const MAX_SINK_COUNT: u8 = 2;

// True if the instruction is a control instruction; so a partly serializing instruction (no other instructions)
// A control instruction gets issued into the rob, but it will prevent the next instruction to be issued, so
// That the branch condition can be determined.
pub(crate) const INSTR_FLAG_IS_BRANCH: u8 = 0;
pub(crate) const INSTR_FLAG_SB_SYNC: u8 = 1;
pub(crate) const INSTR_FLAG_ROB_SYNC: u8 = 2;

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

// #[derive(Debug, Clone, Copy)]
// pub struct Instr {
//     pub cycles: u8,
//     pub opcode: Opcode,
//     pub source_cnt: u8,
//     pub source: [Operand; MAX_SOURCE_COUNT as usize],
//     pub sink_cnt: u8,
//     pub sink: [Operand; MAX_SINK_COUNT as usize],
//     pub loc: Option<SourceLocation>,
//     pub mem_stores: u8,
//     pub flags: u8,
//     pub condition_code: ConditionCode,
// }

// impl Instr {
//     pub(crate) fn is_branch(&self) -> bool {
//         (self.flags & (1 << INSTR_FLAG_IS_BRANCH)) != 0
//     }
//
//     pub(crate) fn set_branch(&mut self) {
//         self.flags |= 1 << INSTR_FLAG_IS_BRANCH;
//     }
//
//     pub(crate) fn rob_sync(&self) -> bool {
//         (self.flags & (1 << INSTR_FLAG_ROB_SYNC)) != 0
//     }
//
//     pub(crate) fn set_rob_sync(&mut self) {
//         self.flags |= 1 << INSTR_FLAG_ROB_SYNC;
//     }
//
//     pub(crate) fn sb_sync(&self) -> bool {
//         (self.flags & (1 << INSTR_FLAG_SB_SYNC)) != 0
//     }
//
//     pub(crate) fn set_sb_sync(&mut self) {
//         self.flags |= 1 << INSTR_FLAG_SB_SYNC;
//     }
// }

// impl fmt::Display for Instr {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{} ", mnemonic(self.opcode))?;
//
//         match self.opcode {
//             Opcode::ADD |
//             Opcode::SUB |
//             Opcode::RSB |
//             Opcode::MUL |
//             Opcode::SDIV |
//             Opcode::AND |
//             Opcode::ORR |
//             Opcode::EOR => write!(f, "{}, {}, {}", self.sink[0], self.source[0], self.source[1])?,
//             Opcode::LDR => write!(f, "{}, {}", self.sink[0], self.source[0])?,
//             Opcode::STR => write!(f, "{}, {}", self.source[0], self.sink[0])?,
//             Opcode::MOV => write!(f, "{}, {}", self.sink[0], self.source[1])?,
//             Opcode::NOP => {}
//             Opcode::ADR => write!(f, "{}, {}", self.sink[0], self.source[0])?,
//             Opcode::PRINTR => write!(f, "{}", self.source[0])?,
//             Opcode::RET |
//             Opcode::B |
//             Opcode::BX |
//             Opcode::BL => write!(f, "{}", self.source[0])?,
//             Opcode::CBZ |
//             Opcode::CBNZ => write!(f, "{}, {}", self.source[0], self.source[1])?,
//             Opcode::NEG => write!(f, "{}, {}", self.sink[0], self.source[0])?,
//             Opcode::MVN => write!(f, "{}, {}", self.sink[0], self.source[0])?,
//             Opcode::CMP => write!(f, "{}, {}", self.source[0], self.source[1])?,
//             Opcode::EXIT => {}
//             Opcode::DSB => {}
//             Opcode::BEQ |
//             Opcode::BNE |
//             Opcode::BLT |
//             Opcode::BLE |
//             Opcode::BGT |
//             Opcode::BGE => write!(f, "{}", self.source[0])?,
//         }
//
//         if let Some(loc) = self.loc {
//             write!(f, " ; {}:{}", loc.line, loc.column)?;
//         }
//
//         Ok(())
//     }
// }

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operand {
    Register(RegisterType),
    // The operand is directly specified in the instruction itself.
    Immediate(DWordType),

    // todo: rename to direct?
    Memory(DWordType),

    Code(DWordType),

    MemRegisterIndirect(RegisterType),

    Unused,
}

impl Operand {
    pub fn base_name(&self) -> &str {
        match self {
            Register(_) => "Register",
            Immediate(_) => "Immediate",
            Memory(_) => "Memory",
            Code(_) => "Code",
            Unused => "Unused",
            MemRegisterIndirect(_) => "MemRegisterIndirect",
        }
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register(reg) => {
                match *reg as u16 {
                    FP => write!(f, "FP"),
                    LR => write!(f, "LR"),
                    SP => write!(f, "SP"),
                    PC => write!(f, "PC"),
                    CPSR => write!(f, "CPSR"),
                    _ => write!(f, "R{}", reg),
                }
            }  // Add a comma here
            Immediate(val) => write!(f, "#{}", val),
            Memory(addr) => write!(f, "[{}]", addr),
            Code(addr) => write!(f, "[{}]", addr),
            Memory(addr) => write!(f, "[{}]", addr),
            Unused => write!(f, "Unused"),
            MemRegisterIndirect(reg) => write!(f, "[{}]", Register(*reg)),
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

    pub(crate) fn get_immediate(&self) -> DWordType {
        match self {
            Operand::Immediate(constant) => *constant,
            _ => panic!("Operand is not a Constant but of type {:?}", self),
        }
    }

    pub(crate) fn get_code_address(&self) -> DWordType {
        match self {
            Operand::Code(constant) => *constant,
            _ => panic!("Operand is not a Code but of type {:?}", self),
        }
    }

    pub(crate) fn get_memory_addr(&self) -> DWordType {
        match self {
            Memory(addr) => *addr,
            _ => panic!("Operand is not a Memory but of type {:?}", self),
        }
    }
}

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

