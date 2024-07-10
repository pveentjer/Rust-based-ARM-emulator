use std::fmt::Debug;
use crate::instructions::instructions::{DWordType, RegisterType};

/// The AST for an AssemblyFile
///
/// The reason why I'm using an AST that isn't tied to a particular parser generator is that
/// it decouples the analysis of the assembly file from the parsing. I have used Pest and
/// I'm now using Lalrpop. The latter gives much better parse errors, but I'm still not too
/// excited because trivial things like case insensitivity, linefeeds, comments are not
/// that easy to fix and the documentation isn't particularly helpful.
///
/// Because it is decoupled, it will make it easier to switch to a different parser generator
/// at some point.


#[derive(Debug, Clone)]
pub struct ASTRegisterOperand {
    pub id: RegisterType,
    pub position: usize,
}

#[derive(Debug, Clone)]
pub struct ASTImmediateOperand {
    pub value: u64,
    pub position: usize,
}

#[derive(Debug, Clone)]
pub struct ASTLabelOperand {
    pub label: String,
    pub position: usize,
}

#[derive(Debug, Clone)]
pub struct ASTAddressOfOperand {
    pub label: String,
    pub position: usize,
}

#[derive(Debug, Clone)]
pub struct ASTMemRegisterIndirectOperand {
    pub register: RegisterType,
    pub position: usize,
}

#[derive(Debug, Clone)]
pub enum ASTOperand {
    Register(ASTRegisterOperand),
    Immediate(ASTImmediateOperand),
    Label(ASTLabelOperand),
    AddressOf(ASTAddressOfOperand),
    MemRegisterIndirect(ASTMemRegisterIndirectOperand),
    // Uncomment and add these if needed
    // MemRegIndirectWithOffset(MemRegIndirectWithOffset),
    // MemRegIndirectWithRegOffset(MemRegIndirectWithRegOffset),
    Unused(),
}

// The visitor is a DFS visitor which is good enough for now. If more flexibility is needed
// then the traversal of the visitor could be externalized
impl ASTOperand {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_operand(self)
    }

    pub fn get_register(&self) -> RegisterType {
        match self {
            ASTOperand::Register(reg) => reg.id,
            _ => panic!("Operation is not a Register but of type {:?}", self),
        }
    }

    pub fn get_immediate(&self) -> DWordType {
        match self {
            ASTOperand::Immediate(immediate) => immediate.value,
            _ => panic!("Operand is not a Constant but of type {:?}", self),
        }
    }

    pub fn get_code_address(&self) -> DWordType {
        panic!();
        // match self {
        //     ASTOperand::AddressOf(address_of_operand) => address_of_operand.label,
        //     _ => panic!("Operand is not a Code but of type {:?}", self),
        // }
    }
    //
    // pub fn get_memory_addr(&self) -> DWordType {
    //     match self {
    //         ASTOperand::(addr) => *addr,
    //         _ => panic!("Operand is not a Memory but of type {:?}", self),
    //     }
    // }

}

#[derive(Debug)]
pub struct ASTData {
    pub name: String,
    pub value: u64,
    pub pos: usize,
}

impl ASTData {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_data(self)
    }
}

#[derive(Debug)]
pub struct ASTInstr {
    pub mnemonic: String,
    pub op1: ASTOperand,
    pub op2: ASTOperand,
    pub op3: ASTOperand,
    pub pos: usize,
}

impl ASTInstr {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        if !self.op1.accept(visitor) { return false; }
        if !self.op2.accept(visitor) { return false; }
        if !self.op3.accept(visitor) { return false; }
        visitor.visit_instr(self)
    }
}

// Define the Directive enum
#[derive(Debug)]
pub enum ASTDirective {
    Global(String, usize),
}

impl ASTDirective {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_directive(self)
    }
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

#[derive(Debug)]
pub struct ASTLabel {
    pub name: String,
    pub pos: usize,
}


impl ASTLabel {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_label(self)
    }
}

#[derive(Debug)]
pub enum ASTTextLine {
    Text(ASTInstr),
    Directive(ASTDirective),
    Label(ASTLabel),
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

#[derive(Debug)]
pub struct ASTTextSection {
    pub lines: Vec<ASTTextLine>,
}

impl ASTTextSection {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        for line in &self.lines {
            if !line.accept(visitor) { return false; }
        }
        visitor.visit_text_section(self)
    }
}

#[derive(Debug)]
pub struct ASTDataSection {
    pub lines: Vec<ASTDataLine>,
}

impl ASTDataSection {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        for line in &self.lines {
            if !line.accept(visitor) { return false; }
        }
        visitor.visit_data_section(self)
    }
}

#[derive(Debug)]
pub struct ASTPreamble {
    pub directives: Vec<ASTDirective>,
}

impl ASTPreamble {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        for directive in &self.directives {
            if !directive.accept(visitor) { return false; }
        }
        visitor.visit_preamble(self)
    }
}

#[derive(Debug)]
pub struct ASTAssemblyFile {
    pub preamble: ASTPreamble,
    pub ds_before: Vec<ASTDataSection>,
    pub ts: ASTTextSection,
    pub ds_after: Vec<ASTDataSection>,
}

impl ASTAssemblyFile {
    pub fn accept(&self, visitor: &mut dyn ASTVisitor) -> bool {
        if !self.preamble.accept(visitor) { return false; }

        for section in &self.ds_before {
            if !section.accept(visitor) { return false; }
        }

        if !self.ts.accept(visitor) { return false; }

        for section in &self.ds_after {
            if !section.accept(visitor) { return false; }
        }
        visitor.visit_assembly_file(self)
    }
}

pub trait ASTVisitor {
    fn visit_operand(&mut self, _ast_operand: &ASTOperand) -> bool { true }
    fn visit_data(&mut self, _ast_data: &ASTData) -> bool { true }
    fn visit_instr(&mut self, _ast_instr: &ASTInstr) -> bool { true }
    fn visit_directive(&mut self, _ast_directive: &ASTDirective) -> bool { true }
    fn visit_label(&mut self, _ast_label: &ASTLabel) -> bool { true }
    fn visit_text_section(&mut self, _ast_label: &ASTTextSection) -> bool { true }
    fn visit_text_line(&mut self, _ast_text_line: &ASTTextLine) -> bool { true }
    fn visit_data_section(&mut self, _ast_label: &ASTDataSection) -> bool { true }
    fn visit_data_line(&mut self, _ast_data_line: &ASTDataLine) -> bool { true }
    fn visit_preamble(&mut self, _ast_preamble: &ASTPreamble) -> bool { true }
    fn visit_assembly_file(&mut self, _ast_assembly: &ASTAssemblyFile) -> bool { true }
}