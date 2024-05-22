use std::fmt::{Debug};


pub enum _Operand {
    Register(u64, usize),
    Immediate(u64, usize),
    Label(String, usize),
    Unused(),
    //MemoryAccess(Operand::Register),
}

pub struct _Data{
    pub name: String,
    pub value: u64,
    pub pos: usize,
}

pub struct _Instr {
    pub mnemonic: String,
    pub op1: _Operand,
    pub op2: _Operand,
    pub op3: _Operand,
    pub pos: usize,
}

pub enum _Directive{
    Global(String, usize),
}

pub enum _GlobalDirective{
    Label(String),
    Immediate()
}

pub enum _DataLine{
    Data(_Data),
    Directive(_Directive),
}

pub struct _Label {
    pub name: String,
    pub pos: usize,
}

pub enum _TextLine{
    Text(_Instr),
    Directive(_Directive),
    Label(_Label),
}

pub enum _Section{
    Text(Vec<_TextLine>),
    Data(Vec<_DataLine>),
}

pub struct _Preamble {
    pub directives: Vec<_Directive>,
}

pub struct _Assembly {
    pub preamble: _Preamble,
    pub sections: Vec<_Section>,
}
