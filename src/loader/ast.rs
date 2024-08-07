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
    pub register: RegisterType,
    pub pos: usize,
}

#[derive(Debug, Clone)]
pub struct ASTImmediateOperand {
    pub value: u64,
    pub pos: usize,
}

#[derive(Debug, Clone)]
pub struct ASTLabelOperand {
    pub label: String,
    pub offset: DWordType,
    pub pos: usize,
}

#[derive(Debug, Clone)]
pub struct ASTAddressOfOperand {
    pub label: String,
    pub pos: usize,
    pub offset: DWordType,
}

#[derive(Debug, Clone)]
pub struct ASTMemRegisterIndirectOperand {
    pub register: RegisterType,
    pub pos: usize,
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

impl ASTOperand {
    pub fn get_type(&self)->ASTOperandType{
        match self {
            ASTOperand::Register(_) => ASTOperandType::Register,
            ASTOperand::Immediate(_) => ASTOperandType::Immediate,
            ASTOperand::Label(_) => ASTOperandType::Label,
            ASTOperand::AddressOf(_) => ASTOperandType::AddressOf,
            ASTOperand::MemRegisterIndirect(_) => ASTOperandType::MemRegisterIndirect,
            ASTOperand::Unused() => ASTOperandType::Unused,
        }
    }
}

pub enum ASTOperandType{
    Register,
    Immediate,
    Label,
    AddressOf,
    MemRegisterIndirect,
    Unused,
}

impl ASTOperandType {
    pub fn base_name(&self)->&str{
        match  self {
            ASTOperandType::Register => "Register",
            ASTOperandType::Immediate => "Immediate",
            ASTOperandType::Label => "Label",
            ASTOperandType::AddressOf => "AddressOf",
            ASTOperandType::MemRegisterIndirect => "MemRegisterIndirect",
            ASTOperandType::Unused => "Unused",
        }
    }
}

// The visitor is a DFS visitor which is good enough for now. If more flexibility is needed
// then the traversal of the visitor could be externalized
impl ASTOperand {
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
        visitor.visit_operand(self)
    }

    pub fn get_register(&self) -> RegisterType {
        match self {
            ASTOperand::Register(reg) => reg.register,
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
        match self {
            ASTOperand::Label(label) => label.offset,
            ASTOperand::AddressOf(address_of) => address_of.offset,
            _ => panic!("Operand is not a Constant but of type {:?}", self),
        }
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
        for line in &mut self.lines {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
        for line in &mut self.lines {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
        for directive in &mut self.directives {
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
    pub fn accept(&mut self, visitor: &mut dyn ASTVisitor) -> bool {
        if !self.preamble.accept(visitor) { return false; }

        for section in &mut self.ds_before {
            if !section.accept(visitor) { return false; }
        }

        if !self.ts.accept(visitor) { return false; }

        for section in &mut self.ds_after {
            if !section.accept(visitor) { return false; }
        }
        visitor.visit_assembly_file(self)
    }
}

pub trait ASTVisitor {
    fn visit_operand(&mut self, _ast_operand: &mut ASTOperand) -> bool { true }
    fn visit_data(&mut self, _ast_data: &mut ASTData) -> bool { true }
    fn visit_instr(&mut self, _ast_instr: &mut ASTInstr) -> bool { true }
    fn visit_directive(&mut self, _ast_directive: &mut ASTDirective) -> bool { true }
    fn visit_label(&mut self, _ast_label: &mut ASTLabel) -> bool { true }
    fn visit_text_section(&mut self, _ast_label: &mut ASTTextSection) -> bool { true }
    fn visit_text_line(&mut self, _ast_text_line: &mut ASTTextLine) -> bool { true }
    fn visit_data_section(&mut self, _ast_label: &mut ASTDataSection) -> bool { true }
    fn visit_data_line(&mut self, _ast_data_line: &mut ASTDataLine) -> bool { true }
    fn visit_preamble(&mut self, _ast_preamble: &mut ASTPreamble) -> bool { true }
    fn visit_assembly_file(&mut self, _ast_assembly: &mut ASTAssemblyFile) -> bool { true }
}