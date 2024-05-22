use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use lalrpop_util::ParseError;

use regex::Regex;
use Operand::{Register};
use crate::{assembly, get_line_and_column};

use crate::cpu::{CPUConfig};
use crate::instructions::instructions::{create_instr, Data, get_opcode, get_register, Instr, Operand, Program, SourceLocation, WordType};
use crate::instructions::instructions::Operand::Code;


struct Loader {
    cpu_config: CPUConfig,
    path: String,
    heap_size: WordType,
    code: Vec<Instr>,
    data_section: HashMap::<String, Rc<Data>>,
    labels: HashMap<String, usize>,
    instr_cnt: usize,
    entry_point: usize,
}

pub enum LoadError{
    ParseError(String),
}

impl Loader {
    fn load(&mut self)->Result<Program,LoadError> {
        let path = &self.path;
        let mut input = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                panic!("Error reading file: {}", err);
            }
        };

        if !input.ends_with('\n') {
            input.push('\n');
        }

        let input_str = input.as_str();

        let parse_result = assembly::AssemblyParser::new()
            .parse(input_str);

        match parse_result {
            Ok(_) => {
                println!("Parse success");
            }
            Err(err) => {

                let cause = match err {
                    ParseError::InvalidToken { location } => {
                        let loc = get_line_and_column(input_str, location);
                        format!("Invalid token at  at {}:{}", loc.line, loc.column)
                    }
                    ParseError::UnrecognizedToken { token, expected } => {
                        let loc = get_line_and_column(input_str, token.0);
                        format!("Unrecognized token '{}' at {}:{}. Expected: {:?}", token.1, loc.line, loc.column, expected)
                    }
                    ParseError::ExtraToken { token } => {
                        let loc = get_line_and_column(input_str, token.0);
                        format!("Extra token '{}' at {}:{}", token.1, loc.line, loc.column)
                    }
                    _ => format!("Error: {:?}", err),
                };

                let msg = format!("Failed to load '{}', cause {}",path,cause);

                //
                // let loc = get_line_and_column(input.as_str(), e.)
                // panic!("{}",e);

                return Err(LoadError::ParseError(msg));
            }
        }

        // // first pass
        // match AssemblyParser::parse(Rule::file, &input) {
        //     Ok(parsed) => {
        //         self.first_pass(parsed);
        //     }
        //     Err(err) => {
        //         panic!("Parsing error: {}", err);
        //     }
        // };
        //
        // // second pass
        // match AssemblyParser::parse(Rule::file, &input) {
        //     Ok(parsed) => {
        //         self.second_pass(parsed);
        //     }
        //     Err(err) => {
        //         panic!("Parsing error: {}", err);
        //     }
        // };


        let mut code = Vec::with_capacity(self.code.len());
        for k in 0..self.code.len() {
            code.push(Rc::new(*self.code.get_mut(k).unwrap()));
        }
        Ok(Program { code, data_items: self.data_section.clone(), entry_point: self.entry_point })
    }
    //
    // fn first_pass(&mut self, root: Pairs<Rule>) {
    //     for pair in root {
    //         match pair.as_rule() {
    //             Rule::label => self.parse_label(pair),
    //             Rule::data_line => self.parse_data(pair),
    //             Rule::instr => { self.instr_cnt += 1 }
    //             _ => {}
    //         }
    //     }
    // }
    //
    // fn second_pass(&mut self, root: Pairs<Rule>) {
    //     for pair in root {
    //         match pair.as_rule() {
    //             Rule::label => {}
    //             Rule::data_line => {}
    //             Rule::directive_line => self.parse_directive(pair),
    //             Rule::instr => self.parse_instr(pair),
    //             _ => {}
    //         }
    //     }
    // }
    //
    //
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
    };

    return loader.load();
}
