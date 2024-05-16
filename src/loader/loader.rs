use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use regex::Regex;

use crate::cpu::{ARCH_REG_SP, CPUConfig, GENERAL_ARG_REG_CNT};
use crate::instructions::instructions::{CodeAddressType, create_NOP, Data, get_opcode, Instr, MemoryAddressType, Opcode, Operand, Program, RegisterType, WordType};
use crate::instructions::instructions::Operand::Code;

#[derive(Parser)]
#[grammar = "loader/assembly.pest"]
struct AssemblyParser;

struct Loader {
    cpu_config: CPUConfig,
    path: String,
    heap_size: MemoryAddressType,
    code: Vec<Instr>,
    data_section: HashMap::<String, Rc<Data>>,
    labels: HashMap<String, usize>,
    unresolved_vec: Vec<Unresolved>,
}

struct Unresolved {
    index: usize,
    label: String,
}

impl Loader {
    fn load(&mut self) {
        let path = &self.path;
        let input = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                panic!("Error reading file: {}", err);
            }
        };

        match AssemblyParser::parse(Rule::file, &input) {
            Ok(parsed) => {
                for pair in parsed {
                    match pair.as_rule() {
                        Rule::assembly => {}
                        Rule::file => {}
                        Rule::EOI => {}
                        Rule::data_section => {}
                        Rule::data => self.parse_data(pair),
                        Rule::label => self.parse_label(pair),
                        Rule::instr_ADD => self.parse_register_bi_instr(pair, Opcode::ADD),
                        Rule::instr_SUB => self.parse_register_bi_instr(pair, Opcode::SUB),
                        Rule::instr_MUL => self.parse_register_bi_instr(pair, Opcode::MUL),
                        Rule::instr_NEG => self.parse_reg_mono_instr(pair, Opcode::NEG),
                        Rule::instr_AND => self.parse_register_bi_instr(pair, Opcode::AND),
                        Rule::instr_ORR => self.parse_register_bi_instr(pair, Opcode::ORR),
                        Rule::instr_EOR => self.parse_register_bi_instr(pair, Opcode::EOR),
                        Rule::instr_NOT => self.parse_reg_self_instr(pair, Opcode::NOT),
                        Rule::instr_NOP => self.parse_NOP(pair),
                        Rule::instr_EXIT => self.parse_EXIT(pair),
                        Rule::instr_MOV => self.parse_reg_mono_instr(pair, Opcode::MOV),
                        Rule::instr_PRINTR => self.parse_PRINTR(pair),
                        Rule::instr_LDR => self.parse_LDR(pair),
                        Rule::instr_STR => self.parse_STR(pair),
                        Rule::instr_PUSH => self.parse_PUSH(pair),
                        Rule::instr_POP => self.parse_POP(pair),
                        Rule::instr_JNZ => self.parse_cond_jump(pair, Opcode::JNZ),
                        Rule::instr_JZ => self.parse_cond_jump(pair, Opcode::JZ),
                        Rule::instr_CALL => self.parse_CALL(pair),
                        Rule::instr_RET => self.parse_RET(pair),
                        _ => panic!("Unknown rule encountered: '{:?}'", pair.as_rule())
                    }
                }
            }
            Err(err) => {
                panic!("Parsing error: {}", err);
                //  eprintln!("Parsing error: {}", err);
            }
        };

        for unresolved in &self.unresolved_vec {
            let mut instr = &mut self.code[unresolved.index];
            if let Some(&address) = self.labels.get(&unresolved.label) {
                for source_index in 0..instr.source_cnt as usize {
                    let source = &mut instr.source[source_index as usize];
                    if let Operand::Code(code_address) = source {
                        if *code_address == 0 {
                            instr.source[source_index] = Code(address as CodeAddressType);
                        }
                    }
                }
            } else {
                panic!("Can't find label {} for instruction at line {}", unresolved.label, instr.line);
            }
        }
    }

    fn parse_label(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let mut label = String::from(inner_pairs.next().unwrap().as_str());
        // get rid of the colon
        //label.pop();

        println!("Label {}", label);

        if self.labels.contains_key(&label) {
            panic!("Duplicate label '{}' at [{}:{}]", label, line_column.0, line_column.1);
        } else {
            self.labels.insert(label, self.code.len());
        }
    }

    fn parse_register_bi_instr(&mut self, pair: Pair<Rule>, opcode: Opcode) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();
        let sink = self.parse_register(&inner_pairs.next().unwrap());
        let src_1 = Operand::Register(self.parse_register(&inner_pairs.next().unwrap()));
        let src2_pair = &inner_pairs.next().unwrap();
        let src2 = match src2_pair.as_rule() {
            Rule::register => Operand::Register(self.parse_register(src2_pair)),
            Rule::immediate => Operand::Immediate(self.parse_immediate(src2_pair)),
            _ => panic!("Unknown rule encountered")
        };

        let line = line_column.0 as i32;
        self.code.push(Instr {
            cycles: 1,
            opcode,
            source_cnt: 2,
            source: [src_1, src2, Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Register(sink), Operand::Unused],
            line,
            mem_stores: 0,
        });
    }

    fn parse_reg_self_instr(&mut self, pair: Pair<Rule>, opcode: Opcode) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();
        let reg = self.parse_register(&inner_pairs.next().unwrap());
        let line = line_column.0 as i32;
        self.code.push(Instr {
            cycles: 1,
            opcode,
            source_cnt: 1,
            source: [Operand::Register(reg), Operand::Unused, Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Register(reg), Operand::Unused],
            line,
            mem_stores: 0,
        });
    }

    fn parse_reg_mono_instr(&mut self, pair: Pair<Rule>, opcode: Opcode) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();
        let dst = self.parse_register(&inner_pairs.next().unwrap());

        let src_pair = &inner_pairs.next().unwrap();
        let src = match src_pair.as_rule() {
            Rule::register => Operand::Register(self.parse_register(src_pair)),
            Rule::immediate => Operand::Immediate(self.parse_immediate(src_pair)),
            _ => panic!("Unknown rule encountered")
        };

        let line = line_column.0 as i32;
        self.code.push(Instr {
            cycles: 1,
            opcode,
            source_cnt: 1,
            source: [src, Operand::Unused, Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Register(dst), Operand::Unused],
            line,
            mem_stores: 0,
        });
    }

    fn parse_STR(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let register = self.parse_register(&inner_pairs.next().unwrap());
        let name = self.parse_variable_reference(&inner_pairs.next().unwrap());

        let data_option = self.data_section.get(&name);
        if data_option.is_none() {
            panic!("Unknown variable '{}' at [{}:{}]", name, line_column.0, line_column.1);
        }

        let data = data_option.unwrap();
        let src = register as RegisterType;
        let addr = data.offset;
        let line = line_column.0 as i32;
        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::STR,
            source_cnt: 1,
            source: [Operand::Register(src), Operand::Unused, Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Memory(addr), Operand::Unused],
            line,
            mem_stores: 1,
        });
    }


    fn parse_LDR(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let register = self.parse_register(&inner_pairs.next().unwrap());
        let variable_or_register = self.parse_variable_reference(&inner_pairs.next().unwrap());

        let data_option = self.data_section.get(&variable_or_register);
        if data_option.is_none() {
            panic!("Unknown variable '{}' at [{}:{}]", variable_or_register, line_column.0, line_column.1);
        }

        let data = data_option.unwrap();
        let addr = data.offset;
        let sink = register as RegisterType;
        let line = line_column.0 as i32;
        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::LDR,
            source_cnt: 1,
            source: [Operand::Memory(addr), Operand::Unused, Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Register(sink), Operand::Unused],
            line,
            mem_stores: 0,
        });
    }

    fn parse_PRINTR(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let reg = self.parse_register(&inner_pairs.next().unwrap());
        let line = line_column.0 as i32;
        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::PRINTR,
            source_cnt: 1,
            source: [Operand::Register(reg), Operand::Unused, Operand::Unused],
            sink_cnt: 0,
            sink: [Operand::Unused, Operand::Unused],
            line,
            mem_stores: 0,
        });
    }

    fn parse_PUSH(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();


        let register = self.parse_register(&inner_pairs.next().unwrap());

        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::PUSH,
            source_cnt: 2,
            source: [Operand::Register(register), Operand::Register(ARCH_REG_SP), Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Register(ARCH_REG_SP), Operand::Unused],
            line: line_column.0 as i32,
            mem_stores: 0,
        });
    }

    fn parse_POP(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();


        let register = self.parse_register(&inner_pairs.next().unwrap());

        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::POP,
            source_cnt: 1,
            source: [Operand::Register(ARCH_REG_SP), Operand::Unused, Operand::Unused],
            sink_cnt: 2,
            sink: [Operand::Register(register), Operand::Register(ARCH_REG_SP)],
            line: line_column.0 as i32,
            mem_stores: 0,
        });
    }

    fn parse_CALL(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let label = String::from(inner_pairs.next().unwrap().as_str());

        let address = match self.labels.get(&label) {
            Some(code_address) => *code_address,
            None => {
                self.unresolved_vec.push(Unresolved { index: self.code.len(), label: label.clone() });
                0
            }
        };

        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::CALL,
            source_cnt: 2,
            source: [Operand::Register(ARCH_REG_SP), Operand::Code(address as CodeAddressType), Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Register(ARCH_REG_SP), Operand::Unused],
            line: line_column.0 as i32,
            mem_stores: 0,
        });
    }

    fn parse_RET(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);

        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::RET,
            source_cnt: 1,
            source: [Operand::Register(ARCH_REG_SP), Operand::Unused, Operand::Unused],
            sink_cnt: 1,
            sink: [Operand::Register(ARCH_REG_SP), Operand::Unused],
            line: line_column.0 as i32,
            mem_stores: 0,
        });
    }

    fn parse_NOP(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        self.code.push(create_NOP(line_column.0 as i32));
    }


    fn parse_EXIT(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        self.code.push(Instr {
            cycles: 1,
            opcode: Opcode::EXIT,
            source_cnt: 0,
            source: [Operand::Unused, Operand::Unused, Operand::Unused],
            sink_cnt: 0,
            sink: [Operand::Unused, Operand::Unused],
            line: line_column.0 as i32,
            mem_stores: 0,
        });
    }

    fn parse_cond_jump(&mut self, pair: Pair<Rule>, opcode: Opcode) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let register = self.parse_register(&inner_pairs.next().unwrap());

        let label = String::from(inner_pairs.next().unwrap().as_str());

        let address = match self.labels.get(&label) {
            Some(code_address) => *code_address,
            None => {
                self.unresolved_vec.push(Unresolved { index: self.code.len(), label: label.clone() });
                0
            }
        };

        self.code.push(Instr {
            cycles: 1,
            opcode,
            source_cnt: 2,
            source: [Operand::Register(register), Operand::Code(address as CodeAddressType), Operand::Unused],
            sink_cnt: 0,
            sink: [Operand::Unused, Operand::Unused],
            line: line_column.0 as i32,
            mem_stores: 0,
        });
    }

    fn parse_data(&mut self, pair: Pair<Rule>) {
        let mut inner_pairs = pair.into_inner();
        let var_pair = inner_pairs.next().unwrap();
        let line_column = self.get_line_column(&var_pair);
        let value_pair = inner_pairs.next().unwrap();

        let variable_name = String::from(var_pair.as_str());
        if !is_valid_variable_name(&variable_name) {
            panic!("Illegal variable name '{}' at [{}:{}]", variable_name, line_column.0, line_column.1);
        }

        let value: i64 = self.parse_integer(&value_pair);
        if self.data_section.contains_key(&variable_name) {
            panic!("Duplicate variable declaration '{}' at [{}:{}]", variable_name, line_column.0, line_column.1);
        }
        self.data_section.insert(variable_name.clone(), Rc::new(Data { value, offset: self.heap_size }));
        self.heap_size += 1;
    }

    fn get_line_column(&mut self, pair: &Pair<Rule>) -> (usize, usize) {
        let start_pos = pair.as_span().start_pos();
        let (line, column) = start_pos.line_col();
        (line, column)
    }

    fn parse_integer(&mut self, pair: &Pair<Rule>) -> i64 {
        pair.as_str().trim().parse().unwrap()
    }

    fn parse_register(&mut self, pair: &Pair<Rule>) -> u16 {
        let line_column = self.get_line_column(&pair);
        let s = pair.as_str();
        if s == "sp" {
            return ARCH_REG_SP;
        } else {
            let reg_name = &s[1..];
            let reg = reg_name.parse().unwrap();
            if reg >= GENERAL_ARG_REG_CNT {
                panic!("Illegal register '{}' at [{}:{}]", &s, line_column.0, line_column.1);
            }
            return reg;
        }
    }

    fn parse_immediate(&mut self, pair: &Pair<Rule>) -> WordType {
        let s = pair.as_str();
        return s[1..].parse().unwrap();
    }

    fn parse_variable_reference(&mut self, pair: &Pair<Rule>) -> String {
        let s = String::from(pair.as_str());
        let s_len = s.len();
        let variable_name = &s[1..s_len - 1];
        return String::from(variable_name);
    }
}

fn is_valid_variable_name(name: &String) -> bool {
    if name.len() == 0 {
        return false;
    }

    let re = Regex::new(r"^(?i)R\d+$").unwrap();
    if re.is_match(name) {
        return false;
    }

    if get_opcode(name).is_some() {
        // It can't be an existing mnemonic
        return false;
    }

    return true;
}

// for the time being we always return the same program
pub fn load(cpu_config: CPUConfig, path: &str) -> Program {
    let mut loader = Loader {
        heap_size: 0,
        cpu_config,
        path: String::from(path),
        code: Vec::new(),
        data_section: HashMap::<String, Rc<Data>>::new(),
        labels: HashMap::<String, usize>::new(),
        unresolved_vec: Vec::new(),
    };

    loader.load();

    let mut code = Vec::with_capacity(loader.code.len());
    for k in 0..loader.code.len() {
        let instr = *loader.code.get(k).unwrap();
        code.push(Rc::new(instr));
    }

    println!("code.len: {}", code.len());

    return Program { code, data_items: loader.data_section.clone() };
}
