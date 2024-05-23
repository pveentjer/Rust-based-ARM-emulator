use std::fmt::{Debug};

#[derive(Debug)]
pub enum ASTOperand {
    // register, position
    Register(u64, usize),
    // value, position
    Immediate(u64, usize),
    // label name, position
    Label(String, usize),
    // the name of the variable
    AddressOf(String, usize),
    // register, offset, position
    MemoryAccess(u64, usize),
    // MemoryAccessWithImmediate(u64, u64, usize),
    // MemoryAccessWithRegister(u64, u64, usize),
    Unused(),
}

#[derive(Debug)]
pub struct ASTData {
    pub name: String,
    pub value: u64,
    pub pos: usize,
}

#[derive(Debug)]
pub struct ASTInstr {
    pub mnemonic: String,
    pub op1: ASTOperand,
    pub op2: ASTOperand,
    pub op3: ASTOperand,
    pub pos: usize,
}

// Define the Directive enum
#[derive(Debug)]
pub enum ASTDirective {
    Global(String, usize),
}

#[derive(Debug)]
pub enum ASTGlobalDirective {
    Label(String),
    Immediate(),
}

#[derive(Debug)]
pub enum ASTDataLine {
    Data(ASTData),
    Directive(ASTDirective),
}

#[derive(Debug)]
pub struct ASTLabel {
    pub name: String,
    pub pos: usize,
}

#[derive(Debug)]
pub enum ASTTextLine {
    Text(ASTInstr),
    Directive(ASTDirective),
    Label(ASTLabel),
}

#[derive(Debug)]
pub struct ASTTextSection {
    pub lines: Vec<ASTTextLine>,
}

#[derive(Debug)]
pub struct ASTDataSection {
    pub lines: Vec<ASTDataLine>,
}

#[derive(Debug)]
pub struct ASTPreamble {
    pub directives: Vec<ASTDirective>,
}

#[derive(Debug)]
pub struct ASTAssemblyFile {
    pub preamble: ASTPreamble,
    pub ds_before: Vec<ASTDataSection>,
    pub ts: ASTTextSection,
    pub ds_after: Vec<ASTDataSection>,
}

pub trait ASTVisitor {
    fn visit_operand(&mut self, ast_operand: &ASTOperand) -> bool { true }
    fn visit_data(&mut self, ast_data: &ASTData) -> bool { true }
    fn visit_instr(&mut self, ast_instr: &ASTInstr) -> bool { true }
    fn visit_directive(&mut self, ast_directive: &ASTDirective) -> bool { true }
    fn visit_label(&mut self, ast_label: &ASTLabel) -> bool { true }
    fn visit_text_section(&mut self, ast_label: &ASTTextSection) -> bool { true }
    fn visit_text_line(&mut self, ast_text_line: &ASTTextLine) -> bool { true }
    fn visit_data_section(&mut self, ast_label: &ASTDataSection) -> bool { true }
    fn visit_data_line(&mut self, ast_data_line: &ASTDataLine) -> bool { true }
    fn visit_preamble(&mut self, ast_preamble: &ASTPreamble) -> bool { true }
    fn visit_assembly_file(&mut self, ast_assembly: &ASTAssemblyFile) -> bool { true }
}

// Implement accept methods for each type
impl ASTOperand {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_operand(self)
    }
}

impl ASTData {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_data(self)
    }
}

impl ASTInstr {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        if !self.op1.accept(visitor) { return false; }
        if !self.op2.accept(visitor) { return false; }
        if !self.op3.accept(visitor) { return false; }
        visitor.visit_instr(self)
    }
}

impl ASTDirective {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_directive(self)
    }
}

impl ASTLabel {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_label(self)
    }
}

impl ASTTextLine {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        let result = match self {
            ASTTextLine::Text(instr) => instr.accept(visitor),
            ASTTextLine::Directive(directive) => directive.accept(visitor),
            ASTTextLine::Label(label) => label.accept(visitor),
        };
        if !result { return false; }
        visitor.visit_text_line(self)
    }
}

impl ASTDataLine {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        let result = match self {
            ASTDataLine::Data(data) => data.accept(visitor),
            ASTDataLine::Directive(directive) => directive.accept(visitor),
        };
        if !result { return false; }
        visitor.visit_data_line(self)
    }
}

impl ASTTextSection {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        for line in &self.lines {
            if !line.accept(visitor) { return false; }
        }
        visitor.visit_text_section(self)
    }
}

impl ASTDataSection {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        for line in &self.lines {
            if !line.accept(visitor) { return false; }
        }
        visitor.visit_data_section(self)
    }
}

impl ASTPreamble {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        for directive in &self.directives {
            if !directive.accept(visitor) { return false; }
        }
        visitor.visit_preamble(self)
    }
}

impl ASTAssemblyFile {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        if !self.preamble.accept(visitor) { return false; }

        for section in &self.ds_before {
            if !section.accept(visitor) { return false; }
        }

        if !self.ts.accept(visitor){ return false; }

        for section in &self.ds_after {
            if !section.accept(visitor) { return false; }
        }
        visitor.visit_assembly_file(self)
    }
}