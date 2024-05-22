use std::fmt::{Debug};


pub enum Operand {
    Register(u64, usize),
    Immediate(u64, usize),
    Label(String, usize),
    Unused(),
    //MemoryAccess(Operand::Register),
}

pub struct Data{
    pub name: String,
    pub value: u64,
    pub pos: usize,
}

pub struct Instr {
    pub mnemonic: String,
    pub op1: Operand,
    pub op2: Operand,
    pub op3: Operand,
    pub pos: usize,
}

pub enum Directive{
    Global(String, usize),
}

pub enum GlobalDirective{
    Label(String),
    Immediate()
}

pub enum DataLine{
    Data(Data),
    Directive(Directive),
}

pub struct Label {
    pub name: String,
    pub pos: usize,
}

pub enum TextLine{
    Text(Instr),
    Directive(Directive),
    Label(Label),
}

pub enum Section{
    Text(Vec<TextLine>),
    Data(Vec<DataLine>),
}

pub struct Preamble {
    pub directives: Vec<Directive>,
}

pub struct Assembly {
    pub preamble: Preamble,
    pub sections: Vec<Section>,
}
