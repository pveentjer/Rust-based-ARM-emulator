use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use lalrpop_util::ParseError;

use regex::Regex;

use crate::{assembly};

use crate::cpu::{CPUConfig, GENERAL_ARG_REG_CNT};
use crate::instructions::instructions::{create_instr, Data, get_opcode, get_register, Instr, Operand, Program, RegisterType, SourceLocation, WordType};
use crate::instructions::instructions::Operand::Register;
use crate::loader::ast::{ASTAssembly, ASTData, ASTDirective, ASTInstr, ASTLabel, ASTOperand, ASTSection, ASTTextLine, ASTVisitor};
use crate::loader::loader::LoadError::AnalysisError;


struct Loader {
    cpu_config: CPUConfig,
    path: String,
    heap_size: WordType,
    code: Vec<Instr>,
    data_section: HashMap::<String, Rc<Data>>,
    labels: HashMap<String, usize>,
    instr_cnt: usize,
    entry_point: usize,
    errors: Vec<String>,
    input_string: String,
}

pub enum LoadError {
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

    fn program_generation(&mut self, assembly: &ASTAssembly) {
        for section in &assembly.sections {
            match section {
                ASTSection::Text(text_section) => {
                    for line in text_section {
                        match line {
                            ASTTextLine::Text(instr) => {
                                match instr.op1 {
                                    ASTOperand::Register(reg, pos) => {
                                        println!("register r{}", reg);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
                ASTSection::Data(_) => {}
            }
        }
    }

    fn parse(&mut self) -> Result<ASTAssembly, Result<Program, LoadError>> {
        let x = &self.input_string;
        let parse_result = assembly::AssemblyParser::new()
            .parse(x.as_str());

        let assembly: ASTAssembly = match parse_result {
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
        Ok(assembly)
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

    //
    // fn parse_directive(&mut self, pair: Pair<Rule>) {
    //     let loc = self.get_source_location(&pair);
    //     let mut inner_pairs = pair.into_inner();
    //     let directive_name = inner_pairs.next().unwrap().as_str();
    //
    //     println!("Directive name: {}", directive_name);
    //
    //     match directive_name {
    //         ".global" => {
    //             let target = self.parse_label_ref(&inner_pairs.next().unwrap());
    //             self.entry_point = target;
    //
    //             println!("Setting entry point to {}", target)
    //         }
    //         ".data" => {}
    //         ".text" => {}
    //         _ => panic!("Unknown directive '{}'at  {},{} ", directive_name, loc.line, loc.column)
    //     }
    // }
    //
    // fn parse_label(&mut self, pair: Pair<Rule>) {
    //     let loc = self.get_source_location(&pair);
    //     let mut inner_pairs = pair.into_inner();
    //
    //     let label = String::from(inner_pairs.next().unwrap().as_str());
    //
    //     if self.labels.contains_key(&label) {
    //         panic!("Duplicate label '{}' at {}:{}", label, loc.line, loc.column);
    //     } else {
    //         self.labels.insert(label, self.instr_cnt);
    //     }
    // }
    //
    // fn parse_instr(&mut self, pair: Pair<Rule>) {
    //     println!("Parse instr");
    //
    //     let loc = self.get_source_location(&pair);
    //     let mut inner_pairs = pair.into_inner();
    //
    //     let mnemonic = inner_pairs.next().unwrap().as_str();
    //     let opcode_option = get_opcode(mnemonic);
    //
    //     if opcode_option.is_none() {
    //         panic!("Unknown mnemonic '{}' at {}:{}", mnemonic, loc.line, loc.column);
    //     }
    //
    //     let opcode = opcode_option.unwrap();
    //
    //     let mut operands = Vec::new();
    //     for operand_pair in inner_pairs {
    //         operands.push(self.parse_operand(&operand_pair));
    //     }
    //
    //     match create_instr(opcode, &operands, loc){
    //         Ok(instr) => {
    //             self.code.push(instr);
    //         }
    //         Err(msg) => {
    //             panic!("{} at {}:{}", msg, loc.line, loc.column);
    //         }
    //     }
    // }
    //
    // fn parse_operand(&self, pair: &Pair<Rule>) -> Operand {
    //     let loc = self.get_source_location(&pair);
    //     let s = pair.as_str().to_lowercase();
    //     match pair.as_rule() {
    //         Rule::register => Register(self.parse_register(pair)),
    //         Rule::immediate => Operand::Immediate(self.parse_immediate(pair)),
    //         Rule::memory_access => Operand::Memory(self.parse_memory_access(pair)),
    //         Rule::variable_address => Operand::Memory(self.parse_variable_address(pair)),
    //         Rule::label_name => Code(self.parse_label_ref(pair) as WordType),
    //         _ => panic!("Unknown operand encountered {} at  at {}:{}", s, loc.line, loc.column),
    //     }
    // }
    //
    // fn parse_register(&self, pair: &Pair<Rule>) -> u16 {
    //     let loc = self.get_source_location(&pair);
    //     let name = pair.as_str();
    //     match get_register(name) {
    //         None => panic!("Illegal register '{}' at {}:{}", name, loc.line, loc.column),
    //         Some(reg) => reg,
    //     }
    // }
    //
    // fn parse_immediate(&self, pair: &Pair<Rule>) -> WordType {
    //     pair.as_str()[1..].parse().unwrap()
    // }
    //
    // fn parse_memory_access(&self, pair: &Pair<Rule>) -> WordType {
    //     let inner_pairs = pair.clone().into_inner();
    //     let base_register = self.parse_register(&inner_pairs.clone().next().unwrap());
    //
    //     if inner_pairs.clone().count() > 1 {
    //         let offset_pair = inner_pairs.clone().nth(1).unwrap();
    //         let offset = match offset_pair.as_rule() {
    //             Rule::register => self.parse_register(&offset_pair) as i64,
    //             Rule::immediate => self.parse_immediate(&offset_pair) as i64,
    //             _ => panic!("Unknown memory access offset"),
    //         };
    //         (base_register as i64 + offset) as WordType
    //     } else {
    //         base_register as WordType
    //     }
    // }
    //
    // fn parse_variable_address(&self, pair: &Pair<Rule>) -> WordType {
    //     let variable_name = pair.as_str()[1..].to_string();
    //     let loc = self.get_source_location(&pair);
    //
    //     if let Some(data) = self.data_section.get(&variable_name) {
    //         data.offset as WordType
    //     } else {
    //         panic!("Unknown variable '{}' at {}:{}", variable_name, loc.line, loc.column);
    //     }
    // }
    //
    // fn parse_data(&mut self, pair: Pair<Rule>) {
    //     let mut inner_pairs = pair.into_inner();
    //     let var_pair = inner_pairs.next().unwrap();
    //     let loc = self.get_source_location(&var_pair);
    //     let value_pair = inner_pairs.next().unwrap();
    //
    //     let variable_name = String::from(var_pair.as_str());
    //     if !is_valid_variable_name(&variable_name) {
    //         panic!("Illegal variable name '{}' at {}:{}", variable_name, loc.line, loc.column);
    //     }
    //
    //     let value: i64 = self.parse_integer(&value_pair);
    //     if self.data_section.contains_key(&variable_name) {
    //         panic!("Duplicate variable declaration '{}' at {}:{}", variable_name, loc.line, loc.column);
    //     }
    //     self.data_section.insert(variable_name.clone(), Rc::new(Data { value, offset: self.heap_size as u64 }));
    //     self.heap_size += 1;
    // }
    //
    // fn get_source_location(&self, pair: &Pair<Rule>) -> SourceLocation {
    //     let start_pos = pair.as_span().start_pos();
    //     let line_col = start_pos.line_col();
    //     return SourceLocation{line:line_col.0, column:line_col.1};
    // }
    //
    // fn parse_integer(&self, pair: &Pair<Rule>) -> i64 {
    //     pair.as_str().trim().parse().unwrap()
    // }
    //
    // fn parse_label_ref(&self, pair: &Pair<Rule>) -> usize {
    //     let loc = self.get_source_location(&pair);
    //     let label = String::from(pair.as_str());
    //
    //     match self.labels.get(&label) {
    //         Some(code_address) => *code_address,
    //         None => {
    //             panic!("Unknown label '{}' at {}:{}", label, loc.line, loc.column)
    //         }
    //     }
    // }
}


pub struct SymbolScan<'a> {
    loader: &'a mut Loader,
}

impl ASTVisitor for SymbolScan<'_> {
    fn visit_data(&mut self, ast_data: &ASTData) -> bool {
        if !is_valid_variable_name(&ast_data.name) {
            let loc = self.loader.to_source_location(ast_data.pos);
            self.loader.errors.push(format!("Illegal variable name '{}' at {}:{}", ast_data.name, loc.line, loc.column));
        }

        if self.loader.data_section.contains_key(&ast_data.name) {
            let loc = self.loader.to_source_location(ast_data.pos);
            self.loader.errors.push(format!("Duplicate variable '{}' at {}:{}", ast_data.name, loc.line, loc.column));
        }

        self.loader.data_section.insert(ast_data.name.clone(),
                                        Rc::new(Data { value: ast_data.value as WordType, offset: self.loader.heap_size as u64 }));
        self.loader.heap_size += 1;
        true
    }

    fn visit_instr(&mut self, _: &ASTInstr) -> bool {
        self.loader.instr_cnt += 1;
        true
    }

    fn visit_label(&mut self, ast_label: &ASTLabel) -> bool {
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
            ASTOperand::Unused() => {}
        };

        true
    }

    fn visit_instr(&mut self, ast_instr: &ASTInstr) -> bool {
        println!("Parse instr");

        // todo: this is very inefficient because for every instruction we scan the whole file
        let loc = self.loader.to_source_location(ast_instr.pos);
        let opcode_option = get_opcode(&ast_instr.mnemonic);

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
pub fn load(cpu_config: CPUConfig, path: &str) -> Result<Program, LoadError> {
    let mut loader = Loader {
        heap_size: 0,
        cpu_config,
        path: String::from(path),
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
