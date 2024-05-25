use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use lalrpop_util::ParseError;

use regex::Regex;

use crate::{assembly};

use crate::cpu::{CPUConfig, GENERAL_ARG_REG_CNT};
use crate::instructions::instructions::{create_instr, Data, get_opcode, Instr, Opcode, Operand, Program, RegisterType, SourceLocation, WordType};
use crate::instructions::instructions::Operand::Register;
use crate::loader::ast::{ASTAssemblyFile, ASTData, ASTDirective, ASTInstr, ASTLabel, ASTOperand, ASTVisitor};
use crate::loader::loader::LoadError::AnalysisError;


struct Loader {
    cpu_config: CPUConfig,
    path: String,
    heap_limit: u32,
    code: Vec<Instr>,
    data_section: HashMap::<String, Rc<Data>>,
    labels: HashMap<String, usize>,
    instr_cnt: usize,
    entry_point: usize,
    errors: Vec<String>,
    input_string: String,
}

pub enum LoadError {
    NotFoundError(String),
    ParseError(String),
    AnalysisError(Vec<String>),
}

impl Loader {
    fn load(&mut self) -> Result<Program, LoadError> {
        let mut input = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(err) => {
                panic!("Error reading file: {}", err);
            }
        };

        if !input.ends_with('\n') {
            input.push('\n');
        }

        self.input_string = input;

        let assembly = match self.parse() {
            Ok(value) => value,
            Err(error) => return error,
        };

        let mut symbolic_scan = SymbolScan { loader: self };
        assembly.accept(&mut symbolic_scan);

        let mut program_generation = ProgramGeneration { loader: self, operand_stack: Vec::new() };
        assembly.accept(&mut program_generation);

        let mut code = Vec::with_capacity(self.code.len());
        for k in 0..self.code.len() {
            code.push(Rc::new(*self.code.get_mut(k).unwrap()));
        }

        return if self.errors.is_empty() {
            Ok(Program { code, data_items: self.data_section.clone(), entry_point: self.entry_point })
        } else {
            Err(AnalysisError(self.errors.clone()))
        };
    }

    fn parse(&mut self) -> Result<ASTAssemblyFile, Result<Program, LoadError>> {
        let x = &self.input_string;
        let parse_result = assembly::AssemblyFileParser::new()
            .parse(x.as_str());

        let assembly_file: ASTAssemblyFile = match parse_result {
            Ok(a) => a,
            Err(err) => {
                let cause = match err {
                    ParseError::InvalidToken { location } => {
                        let loc = self.to_source_location(location);
                        format!("Invalid token at {}:{}", loc.line, loc.column)
                    }
                    ParseError::UnrecognizedToken { token, expected } => {
                        let loc = self.to_source_location(token.0);
                        format!("Unrecognized token '{}' at {}:{}. Expected: {}", token.1, loc.line, loc.column, expected.join(" or "))
                    }
                    ParseError::ExtraToken { token } => {
                        let loc = self.to_source_location(token.0);
                        format!("Extra token '{}' at {}:{}", token.1, loc.line, loc.column)
                    }
                    _ => format!("{:?}", err),
                };

                return Err(Err(LoadError::ParseError(cause)));
            }
        };
        Ok(assembly_file)
    }

    fn to_source_location(&self, offset: usize) -> SourceLocation {
        let mut line = 1;
        let mut col = 1;
        let string = &self.input_string;
        let input = string.as_str();
        for (i, c) in input.char_indices() {
            if i == offset {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        SourceLocation { line: line, column: col }
    }
}


pub struct SymbolScan<'a> {
    loader: &'a mut Loader,
}

impl ASTVisitor for SymbolScan<'_> {
    fn visit_data(&mut self, ast_data: &ASTData) -> bool {
        if self.loader.heap_limit == self.loader.cpu_config.memory_size {
            let loc = self.loader.to_source_location(ast_data.pos);
            self.loader.errors.push(format!("Insufficient heap to declare variable '{}' at {}:{}", ast_data.name, loc.line, loc.column));
            return false;
        }

        if !is_valid_variable_name(&ast_data.name) {
            let loc = self.loader.to_source_location(ast_data.pos);
            self.loader.errors.push(format!("Illegal variable name '{}' at {}:{}", ast_data.name, loc.line, loc.column));
        }

        if self.loader.labels.contains_key(&ast_data.name) {
            let loc = self.loader.to_source_location(ast_data.pos);
            self.loader.errors.push(format!("There already exists a label with name '{}' at {}:{}", ast_data.name, loc.line, loc.column));
        }

        if self.loader.data_section.contains_key(&ast_data.name) {
            let loc = self.loader.to_source_location(ast_data.pos);
            self.loader.errors.push(format!("Duplicate variable '{}' at {}:{}", ast_data.name, loc.line, loc.column));
        }

        self.loader.data_section.insert(ast_data.name.clone(),
                                        Rc::new(Data { value: ast_data.value as WordType, offset: self.loader.heap_limit as u64 }));
        self.loader.heap_limit += 1;
        true
    }

    fn visit_instr(&mut self, _: &ASTInstr) -> bool {
        self.loader.instr_cnt += 1;
        true
    }

    fn visit_label(&mut self, ast_label: &ASTLabel) -> bool {
        if self.loader.data_section.contains_key(&ast_label.name) {
            let loc = self.loader.to_source_location(ast_label.pos);
            self.loader.errors.push(format!("There already exists a variable with name '{}' at {}:{}", ast_label.name, loc.line, loc.column));
        }

        if self.loader.labels.contains_key(&ast_label.name) {
            let loc = self.loader.to_source_location(ast_label.pos);
            self.loader.errors.push(format!("Duplicate label '{}' at {}:{}", ast_label.name, loc.line, loc.column));
        } else {
            self.loader.labels.insert(ast_label.name.clone(), self.loader.instr_cnt);
        }
        true
    }
}

pub struct ProgramGeneration<'a> {
    loader: &'a mut Loader,
    operand_stack: Vec<Operand>,
}

impl ASTVisitor for ProgramGeneration<'_> {
    fn visit_operand(&mut self, ast_operand: &ASTOperand) -> bool {
        match ast_operand {
            ASTOperand::Register(reg, pos) => {
                if *reg >= GENERAL_ARG_REG_CNT as u64 {
                    let loc = self.loader.to_source_location(*pos);
                    self.loader.errors.push(format!("Unknown register r'{}' at {}:{}", *reg, loc.line, loc.column));
                    return false;
                }

                self.operand_stack.push(Register(*reg as RegisterType));
            }
            ASTOperand::Immediate(value, _) => {
                self.operand_stack.push(Operand::Immediate(*value as WordType));
            }
            ASTOperand::Label(label_name, pos) => {
                match self.loader.labels.get(label_name) {
                    Some(code_address) => {
                        self.operand_stack.push(Operand::Code(*code_address as WordType));
                    }
                    None => {
                        let loc = self.loader.to_source_location(*pos);
                        self.loader.errors.push(format!("Unknown label '{}' at {}:{}", label_name, loc.line, loc.column));
                        return false;
                    }
                }
            }
            ASTOperand::AddressOf(label_name, pos) => {
                match self.loader.data_section.get(label_name) {
                    Some(data) => {
                        self.operand_stack.push(Operand::Immediate(data.offset as WordType));
                    }
                    None => {
                        let loc = self.loader.to_source_location(*pos);
                        self.loader.errors.push(format!("Unknown variable '{}' at {}:{}", label_name, loc.line, loc.column));
                        return false;
                    }
                }
            }

            ASTOperand::Unused() => {}
            ASTOperand::MemRegisterIndirect(register, pos) => {
                // address
                self.operand_stack.push(Operand::Register(*register as RegisterType));
                // offset
                //self.operand_stack.push(Operand::Immediate(0));
            }
            //ASTOperand::MemoryAccessWithImmediate(_, _, _) => {}
        };

        true
    }

    fn visit_instr(&mut self, ast_instr: &ASTInstr) -> bool {
        // todo: this is very inefficient because for every instruction we scan the whole file
        let loc = self.loader.to_source_location(ast_instr.pos);
        let mut opcode_option = get_opcode(&ast_instr.mnemonic);

        if opcode_option.is_some() {
            let opcode = opcode_option.unwrap();
            // Exit should not be used in the program
            if opcode == Opcode::EXIT {
                opcode_option = None;
            }
        }

        if opcode_option.is_none() {
            self.loader.errors.push(format!("Unknown mnemonic '{}' at {}:{}", ast_instr.mnemonic, loc.line, loc.column));
            return false;
        }

        let opcode = opcode_option.unwrap();
        match create_instr(opcode, &self.operand_stack, loc) {
            Ok(instr) => {
                self.loader.code.push(instr);
            }
            Err(msg) => {
                self.loader.errors.push(format!("{} at {}:{}", msg, loc.line, loc.column));
            }
        };
        self.operand_stack.clear();
        true
    }

    fn visit_directive(&mut self, ast_directive: &ASTDirective) -> bool {
        match ast_directive {
            ASTDirective::Global(start_label, pos) => {
                match self.loader.labels.get(start_label) {
                    Some(code_address) => {
                        self.loader.entry_point = *code_address as usize;
                        return true;
                    }
                    None => {
                        let loc = self.loader.to_source_location(*pos);
                        self.loader.errors.push(format!("Unknown label '{}' at {}:{}", start_label, loc.line, loc.column));
                        return false;
                    }
                }
            }
        }
    }
}

fn is_valid_variable_name(name: &String) -> bool {
    if name.is_empty() {
        return false;
    }

    // todo: the other registers are ignored.
    let re = Regex::new(r"^(?i)R\d+$").unwrap();
    if re.is_match(name) {
        return false;
    }

    if get_opcode(name).is_some() {
        // It can't be an existing mnemonic
        return false;
    }

    true
}

// for the time being we always return the same program
pub fn load(cpu_config: CPUConfig, path_str: &str) -> Result<Program, LoadError> {
    let path = Path::new(path_str);

    if !path.exists() {
        return Err(LoadError::NotFoundError(format!("File '{}' does not exist.", path_str)));
    }

    let mut loader = Loader {
        heap_limit: 0,
        cpu_config,
        path: String::from(path_str),
        code: Vec::new(),
        data_section: HashMap::<String, Rc<Data>>::new(),
        labels: HashMap::<String, usize>::new(),
        instr_cnt: 0,
        entry_point: 0,
        errors: Vec::new(),
        input_string: String::new(),
    };

    return loader.load();
}
