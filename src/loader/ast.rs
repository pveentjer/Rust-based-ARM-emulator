use std::fmt::{Debug};

// Define the Operand enum
#[derive(Debug)]
pub enum ASTOperand {
    Register(u64, usize),
    Immediate(u64, usize),
    Label(String, usize),
    Unused(),
}

// Define the Data struct
#[derive(Debug)]
pub struct ASTData {
    pub name: String,
    pub value: u64,
    pub pos: usize,
}

// Define the Instr struct
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

// Define the GlobalDirective enum
#[derive(Debug)]
pub enum ASTGlobalDirective {
    Label(String),
    Immediate(),
}

// Define the DataLine enum
#[derive(Debug)]
pub enum ASTDataLine {
    Data(ASTData),
    Directive(ASTDirective),
}

// Define the Label struct
#[derive(Debug)]
pub struct ASTLabel {
    pub name: String,
    pub pos: usize,
}

// Define the TextLine enum
#[derive(Debug)]
pub enum ASTTextLine {
    Text(ASTInstr),
    Directive(ASTDirective),
    Label(ASTLabel),
}

// Define the Section enum
#[derive(Debug)]
pub enum ASTSection {
    Text(Vec<ASTTextLine>),
    Data(Vec<ASTDataLine>),
}

// Define the Preamble struct
#[derive(Debug)]
pub struct ASTPreamble {
    pub directives: Vec<ASTDirective>,
}

// Define the Assembly struct
#[derive(Debug)]
pub struct ASTAssembly {
    pub preamble: ASTPreamble,
    pub sections: Vec<ASTSection>,
}

pub trait ASTVisitor {
    fn visit_operand(&mut self, ast_operand: &ASTOperand) {}
    fn visit_data(&mut self, ast_data: &ASTData) {}
    fn visit_instr(&mut self, ast_instr: &ASTInstr) {}
    fn visit_directive(&mut self, ast_directive: &ASTDirective) {}
    fn visit_label(&mut self, ast_label: &ASTLabel) {}
    fn visit_text_line(&mut self, ast_text_line: &ASTTextLine) {}
    fn visit_data_line(&mut self, ast_data_line: &ASTDataLine) {}
    fn visit_section(&mut self, ast_section: &ASTSection) {}
    fn visit_preamble(&mut self, ast_preamble: &ASTPreamble) {}
    fn visit_assembly(&mut self, ast_assembly: &ASTAssembly) {}
}

// Implement accept methods for each type
impl ASTOperand {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        visitor.visit_operand(self);
    }
}

impl ASTData {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        visitor.visit_data(self);
    }
}

impl ASTInstr {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        self.op1.accept(visitor);
        self.op2.accept(visitor);
        self.op3.accept(visitor);
        visitor.visit_instr(self);
    }
}

impl ASTDirective {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        visitor.visit_directive(self);
    }
}

impl ASTLabel {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        visitor.visit_label(self);
    }
}

impl ASTTextLine {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        match self {
            ASTTextLine::Text(instr) => instr.accept(visitor),
            ASTTextLine::Directive(directive) => directive.accept(visitor),
            ASTTextLine::Label(label) => label.accept(visitor),
        }
        visitor.visit_text_line(self);
    }
}

impl ASTDataLine {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        match self {
            ASTDataLine::Data(data) => data.accept(visitor),
            ASTDataLine::Directive(directive) => directive.accept(visitor),
        }
        visitor.visit_data_line(self);
    }
}

impl ASTSection {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        match self {
            ASTSection::Text(lines) => {
                for line in lines {
                    line.accept(visitor);
                }
            }
            ASTSection::Data(lines) => {
                for line in lines {
                    line.accept(visitor);
                }
            }
        }
        visitor.visit_section(self);
    }
}

impl ASTPreamble {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        for directive in &self.directives {
            directive.accept(visitor);
        }
        visitor.visit_preamble(self);
    }
}

impl ASTAssembly {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) {
        self.preamble.accept(visitor);
        for section in &self.sections {
            section.accept(visitor);
        }
        visitor.visit_assembly(self);
    }
}
