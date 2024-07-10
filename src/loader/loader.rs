use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use lalrpop_util::ParseError;
use regex::Regex;

use crate::assembly;
use crate::cpu::{CPSR, CPUConfig, GENERAL_ARG_REG_CNT, LR};
use crate::instructions::instructions::{Branch, BranchTarget, ConditionCode, Data, DataProcessing, DWordType,
                                        get_opcode, Instr, LoadStore, Opcode, Operand2, Printr, Program, RegisterType,
                                        SourceLocation, Synchronization};
use crate::loader::ast::{ASTAssemblyFile, ASTData, ASTDirective, ASTInstr, ASTLabel, ASTOperand, ASTVisitor};
use crate::loader::loader::LoadError::AnalysisError;

struct Loader {
    cpu_config: CPUConfig,
    src: String,
    heap_limit: u32,
    code: Vec<Instr>,
    data_section: HashMap::<String, Rc<Data>>,
    labels: HashMap<String, usize>,
    instr_cnt: usize,
    entry_point: usize,
    errors: Vec<String>,
}

pub enum LoadError {
    NotFoundError(String),
    IOError(String),
    ParseError(String),
    AnalysisError(Vec<String>),
}

impl Loader {
    fn load(&mut self) -> Result<Program, LoadError> {
        if !self.src.ends_with('\n') {
            self.src.push('\n');
        }

        let mut assembly = match self.parse() {
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
        // todo: ugly variable name
        let x = &self.src;
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
        let src = &self.src;
        let src_slice = src.as_str();
        for (i, c) in src_slice.char_indices() {
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

pub(crate) fn create_instr(
    opcode: Opcode,
    operands: &Vec<ASTOperand>,
    loc: SourceLocation,
) -> Result<Instr, String> {
    let instr = match opcode {
        Opcode::SUB |
        Opcode::MUL |
        Opcode::SDIV |
        Opcode::AND |
        Opcode::ORR |
        Opcode::EOR |
        Opcode::RSB |
        Opcode::ADD => {
            validate_operand_count(3, operands, opcode, loc)?;

            let rd = operands[0].get_register();
            let rn = operands[1].get_register();

            let operand2 = match &operands[2] {
                ASTOperand::Register(register) => Operand2::Register { reg_id: register.register },
                ASTOperand::Immediate(immediate) => Operand2::Immediate { value: immediate.value },
                _ => { panic!() }
            };

            Instr::DataProcessing {
                data_processing: DataProcessing {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rn: Some(rn),
                    rd,
                    rd_read: false,
                    operand2,
                }
            }
        }
        Opcode::MVN |
        Opcode::NEG => {
            validate_operand_count(2, operands, opcode, loc)?;

            let rd = operands[0].get_register();
            let rn = operands[1].get_register();

            Instr::DataProcessing {
                data_processing: DataProcessing {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rn: Some(rn),
                    rd,
                    rd_read: false,
                    operand2: Operand2::Unused(),
                }
            }
        }
        Opcode::TEQ |
        Opcode::TST |
        Opcode::CMP => {
            validate_operand_count(2, operands, opcode, loc)?;

            let rd = CPSR as RegisterType;
            let rn = operands[0].get_register();

            let operand2 = match &operands[1] {
                ASTOperand::Register(register) => Operand2::Register { reg_id: register.register },
                ASTOperand::Immediate(immediate) => Operand2::Immediate { value: immediate.value },
                _ => { panic!() }
            };

            Instr::DataProcessing {
                data_processing: DataProcessing {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rn: Some(rn),
                    rd,
                    rd_read: true,
                    operand2,
                }
            }
        }
        Opcode::ADR => { panic!() }
        Opcode::STR |
        Opcode::LDR => {
            validate_operand_count(2, operands, opcode, loc)?;

            let rd = operands[0].get_register();

            let rn = match &operands[1] {
                ASTOperand::MemRegisterIndirect(mem_register_indirect)
                => mem_register_indirect.register,
                _ => { panic!() }
            };

            Instr::LoadStore {
                load_store: LoadStore {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rd,
                    rn,
                    offset: 0,
                }
            }
        }
        Opcode::PRINTR => {
            validate_operand_count(1, operands, opcode, loc)?;

            let rn = operands[0].get_register();

            Instr::Printr {
                printr: Printr {
                    loc: Some(loc),
                    rn,
                }
            }
        }
        Opcode::MOV => {
            validate_operand_count(2, operands, opcode, loc)?;

            let rd = operands[0].get_register();

            let operand2 = match &operands[1] {
                ASTOperand::Register(register) => Operand2::Register { reg_id: register.register },
                ASTOperand::Immediate(immediate) => Operand2::Immediate { value: immediate.value },
                ASTOperand::AddressOf(address_of) => Operand2::Immediate { value: address_of.offset },
                _ => { panic!("Unhandled {:?}", &operands[1]) }
            };

            Instr::DataProcessing {
                data_processing: DataProcessing {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    rn: None,
                    rd,
                    rd_read: false,
                    operand2,
                }
            }
        }
        Opcode::RET => {
            if operands.len() > 1 {
                return Err(format!("Operand count mismatch. {:?} expects 0 or 1 argument, but {} are provided at {}:{}",
                                   opcode, operands.len(), loc.line, loc.column));
            }

            let target = if operands.len() == 0 {
                LR as RegisterType
            } else {
                operands[0].get_register()
            };

            Instr::Branch {
                branch: Branch {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    link_bit: false,
                    target: BranchTarget::Register { register: target },
                    rt: None,
                }
            }
        }
        Opcode::B => {
            validate_operand_count(1, operands, opcode, loc)?;

            let offset = operands[0].get_code_address();

            Instr::Branch {
                branch: Branch {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    link_bit: false,
                    target: BranchTarget::Immediate { offset: offset as u32 },
                    rt: None,
                }
            }
        }
        Opcode::BX => {
            validate_operand_count(1, operands, opcode, loc)?;

            let target = operands[0].get_register();

            Instr::Branch {
                branch: Branch {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    link_bit: false,
                    target: BranchTarget::Register { register: target },
                    rt: None,
                }
            }
        }
        Opcode::BL => {
            crate::loader::loader::validate_operand_count(1, operands, opcode, loc)?;

            let offset = operands[0].get_code_address();

            Instr::Branch {
                branch: Branch {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    link_bit: true,
                    target: BranchTarget::Immediate { offset: offset as u32 },
                    rt: None,
                }
            }
        }
        Opcode::CBZ |
        Opcode::CBNZ => {
            crate::loader::loader::validate_operand_count(2, operands, opcode, loc)?;

            let rt = operands[0].get_register();
            let target = operands[1].get_code_address();

            Instr::Branch {
                branch: Branch {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    link_bit: false,
                    target: BranchTarget::Immediate { offset: target as u32 },
                    rt: Some(rt),
                }
            }
        }
        Opcode::NOP |
        Opcode::EXIT |
        Opcode::DSB => {
            validate_operand_count(0, operands, opcode, loc)?;

            Instr::Synchronization {
                synchronization: Synchronization {
                    opcode,
                    loc: Some(loc),
                }
            }
        }
        Opcode::BEQ |
        Opcode::BNE |
        Opcode::BLT |
        Opcode::BLE |
        Opcode::BGT |
        Opcode::BGE => {
            validate_operand_count(1, operands, opcode, loc)?;

            let offset = operands[0].get_code_address();

            Instr::Branch {
                branch: Branch {
                    opcode,
                    condition: ConditionCode::AL,
                    loc,
                    link_bit: false,
                    target: BranchTarget::Immediate { offset: offset as u32 },
                    rt: Some(CPSR),
                }
            }
        }
    };

    // todo: handling of instructions with control like modifying the IP need to be detected.
    //
    // if !instr.is_branch() && has_control_operands(&instr) {
    //     instr.set_branch();
    // }

    return Ok(instr);
}

fn validate_operand_count(expected: usize,
                          operands: &Vec<ASTOperand>,
                          opcode: Opcode,
                          loc: SourceLocation) -> Result<(), String> {
    if operands.len() != expected {
        return Err(format!("Operand count mismatch. {:?} expects {} arguments, but {} are provided at {}:{}",
                           opcode, expected, operands.len(), loc.line, loc.column));
    }
    Ok(())
}

pub struct SymbolScan<'a> {
    loader: &'a mut Loader,
}

impl ASTVisitor for SymbolScan<'_> {
    fn visit_data(&mut self, ast_data: &mut ASTData) -> bool {
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
                                        Rc::new(Data { value: ast_data.value as DWordType, offset: self.loader.heap_limit as u64 }));
        self.loader.heap_limit += 1;
        true
    }

    fn visit_instr(&mut self, _: &mut ASTInstr) -> bool {
        self.loader.instr_cnt += 1;
        true
    }

    fn visit_label(&mut self, ast_label: &mut ASTLabel) -> bool {
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
    operand_stack: Vec<ASTOperand>,
}

impl ASTVisitor for ProgramGeneration<'_> {
    fn visit_operand(&mut self, ast_operand: &mut ASTOperand) -> bool {
        match ast_operand {
            ASTOperand::Register(register) => {
                if register.register >= GENERAL_ARG_REG_CNT as RegisterType {
                    let loc = self.loader.to_source_location(register.pos);
                    self.loader.errors.push(format!("Unknown register r'{}' at {}:{}", register.register, loc.line, loc.column));
                    return false;
                }

                self.operand_stack.push(ast_operand.clone());
            }
            ASTOperand::Immediate(immediate) => {
                self.operand_stack.push(ast_operand.clone());
            }
            ASTOperand::Label(label) => {
                match self.loader.labels.get(&mut label.label) {
                    Some(code_address) => {
                        label.offset = *code_address as DWordType;
                        self.operand_stack.push(ast_operand.clone());
                    }
                    None => {
                        let loc = self.loader.to_source_location(label.pos);
                        self.loader.errors.push(format!("Unknown label '{}' at {}:{}", label.label, loc.line, loc.column));
                        return false;
                    }
                }
            }
            ASTOperand::AddressOf(address_of) => {
                match self.loader.data_section.get(&address_of.label) {
                    Some(data) => {
                        address_of.offset = data.offset as DWordType;
                        self.operand_stack.push(ast_operand.clone());
                    }
                    None => {
                        let loc = self.loader.to_source_location(address_of.pos);
                        self.loader.errors.push(format!("Unknown variable '{}' at {}:{}", address_of.label, loc.line, loc.column));
                        return false;
                    }
                }
            }

            ASTOperand::Unused() => {}
            ASTOperand::MemRegisterIndirect(mem_register_indirect) => {
                self.operand_stack.push(ast_operand.clone());
            }
            //ASTOperand::MemoryAccessWithImmediate(_, _, _) => {}
        };

        true
    }

    fn visit_instr(&mut self, ast_instr: &mut ASTInstr) -> bool {
        // todo: this is very inefficient because for every instruction the whole file content is scanned.
        let loc = self.loader.to_source_location(ast_instr.pos);
        let opcode_option = get_opcode(&ast_instr.mnemonic);

        if opcode_option.is_none() || opcode_option.unwrap() == Opcode::EXIT {
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

    fn visit_directive(&mut self, ast_directive: &mut ASTDirective) -> bool {
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

pub fn load_from_file(cpu_config: CPUConfig, path_str: &str) -> Result<Program, LoadError> {
    let path = Path::new(path_str);

    if !path.exists() {
        return Err(LoadError::NotFoundError(format!("File '{}' does not exist.", path_str)));
    }

    let src = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            return Err(LoadError::IOError(err.to_string()));
        }
    };

    return load_from_string(cpu_config, src);
}

pub fn load_from_string(cpu_config: CPUConfig, src: String) -> Result<Program, LoadError> {
    let mut loader = Loader {
        heap_limit: 0,
        cpu_config,
        src,
        code: Vec::new(),
        data_section: HashMap::<String, Rc<Data>>::new(),
        labels: HashMap::<String, usize>::new(),
        instr_cnt: 0,
        entry_point: 0,
        errors: Vec::new(),
    };

    return loader.load();
}