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

pub enum Directive{
    Global(String),
}

pub enum GlobalDirective{
    Label(String),
    Immediate()
}

pub enum DataLine{
    Data(Data),
    Directive(Directive),
}

pub struct  Label {
    pub name: String,
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

pub struct Assembly{
    pub preamble: Preamble,
    pub sections: Vec<Section>,
}
