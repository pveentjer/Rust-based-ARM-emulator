use std::fmt::{Debug, Error, Formatter};


pub enum Operand {
    Register(u64),
    Immediate(u64),
    //MemoryAccess(Operand::Register),
}

pub struct Data{
    pub name: String,
    pub value: u64,
}

pub struct Instr{
    pub mnemonic: String,
    pub op1: Operand,
    pub op2: Operand,
    pub op3: Operand,
}

pub enum Section{
    Text(Vec<Instr>),
    Data(Vec<Data>),
}

pub struct Assembly{
    pub section: Vec<Section>,
}
