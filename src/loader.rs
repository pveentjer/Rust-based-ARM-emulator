use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use pest::iterators::{Pair};
use pest::Parser;
use pest_derive::Parser;
use regex::Regex;
use crate::cpu::CPUConfig;
use crate::instructions::{CodeAddressType, create_JNZ, create_JZ, create_LOAD, create_reg_bi_Instr, create_NOP, create_PRINTR, create_STORE, Data, Instr, MemoryAddressType, Opcode, Program, RegisterType, create_reg_mono_Instr, Operand, OpType, OpUnion, get_opcode};


#[derive(Parser)]
#[grammar = "assembly.pest"]
struct AssemblyParser;

struct Loader {
    cpu_config: CPUConfig,
    path: String,
    heap_size: MemoryAddressType,
    code: Vec::<Rc<Instr>>,
    data_section: HashMap::<String, Rc<Data>>,
    labels: HashMap<String, usize>,
}


impl Loader {
    fn load(&mut self) -> Program {
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
                        Rule::instr_DIV => self.parse_register_bi_instr(pair, Opcode::DIV),
                        Rule::instr_MOD => self.parse_register_bi_instr(pair, Opcode::MOD),
                        Rule::instr_INC => self.parse_reg_self_instr(pair, Opcode::INC),
                        Rule::instr_DEC => self.parse_reg_self_instr(pair, Opcode::DEC),
                        Rule::instr_NEG => self.parse_reg_self_instr(pair, Opcode::NEG),
                        Rule::instr_AND => self.parse_register_bi_instr(pair, Opcode::AND),
                        Rule::instr_OR => self.parse_register_bi_instr(pair, Opcode::OR),
                        Rule::instr_XOR => self.parse_register_bi_instr(pair, Opcode::XOR),
                        Rule::instr_NOT => self.parse_reg_self_instr(pair, Opcode::NOT),
                        Rule::instr_NOP => self.parse_reg_mono_instr(pair, Opcode::MOV),
                        Rule::instr_MOV => self.parse_reg_mono_instr(pair, Opcode::MOV),
                        Rule::instr_PRINTR => self.parse_PRINTR(pair),
                        Rule::instr_LOAD => self.parse_LOAD(pair),
                        Rule::instr_STORE => self.parse_STORE(pair),
                        Rule::instr_JNZ => self.parse_JNZ(pair),
                        _ => panic!("Unknown rule encountered: '{:?}'", pair.as_rule())
                    }
                }
            }
            Err(err) => {
                panic!("Parsing error: {}", err);
                //  eprintln!("Parsing error: {}", err);
            }
        };
        return Program::new(self.code.clone(), self.data_section.clone());
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
        let instr = create_reg_bi_Instr(
            opcode,
            self.parse_register(&inner_pairs.next().unwrap()),
            self.parse_register(&inner_pairs.next().unwrap()),
            self.parse_register(&inner_pairs.next().unwrap()),
            line_column.0 as i32);
        self.code.push(Rc::new(instr));
    }

    fn parse_reg_self_instr(&mut self, pair: Pair<Rule>, opcode: Opcode) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();
        let reg = self.parse_register(&inner_pairs.next().unwrap());
        let instr = create_reg_mono_Instr(
            opcode,
            reg,
            reg,
            line_column.0 as i32);
        self.code.push(Rc::new(instr));
    }

    fn parse_reg_mono_instr(&mut self, pair: Pair<Rule>, opcode: Opcode) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();
        let instr = create_reg_mono_Instr(
            opcode,
            self.parse_register(&inner_pairs.next().unwrap()),
            self.parse_register(&inner_pairs.next().unwrap()),
            line_column.0 as i32);
        self.code.push(Rc::new(instr));
    }

    fn parse_STORE(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let register = self.parse_register(&inner_pairs.next().unwrap());
        let name = self.parse_variable_reference(&inner_pairs.next().unwrap());

        let data_option = self.data_section.get(&name);
        if data_option.is_none() {
            // todo: add line
            panic!("Unknown variable '{}'", name);
        }

        let data = data_option.unwrap();
        let instr = create_STORE(register as RegisterType, data.offset, line_column.0 as i32);
        self.code.push(Rc::new(instr));
    }

    fn parse_LOAD(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();


        let variable_or_register = self.parse_variable_reference(&inner_pairs.next().unwrap());
        let register = self.parse_register(&inner_pairs.next().unwrap());

        let data_option = self.data_section.get(&variable_or_register);
        if data_option.is_none() {
            // todo: add line
            panic!("Unknown variable '{}'", variable_or_register);
        }

        let data = data_option.unwrap();
        let addr = data.offset;
        let sink = register as RegisterType;
        let line = line_column.0 as i32;
        let instr = Instr {
            cycles: 1,
            opcode: Opcode::LOAD,
            source_cnt: 1,
            source: [
                Operand { op_type: OpType::MEMORY, union: OpUnion::Memory(addr) },
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
            ],
            sink: Operand { op_type: OpType::REGISTER, union: OpUnion::Register(sink) },
            line,
        };
        self.code.push(Rc::new(instr));
    }

    fn parse_PRINTR(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let instr = create_PRINTR(self.parse_register(&inner_pairs.next().unwrap()), line_column.0 as i32);
        self.code.push(Rc::new(instr));
    }

    fn parse_NOP(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let instr = create_NOP(line_column.0 as i32);
        self.code.push(Rc::new(instr))
    }

    fn parse_JNZ(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let register = self.parse_register(&inner_pairs.next().unwrap());

        let label = String::from(inner_pairs.next().unwrap().as_str());

        let code_offset_option = self.labels.get(&label);
        if code_offset_option.is_none() {
            panic!("Unknown label '{}' at {}:{}", label, line_column.0, line_column.1);
        }

        let offset = code_offset_option.unwrap();
        let instr = create_JNZ(
            register,
            *offset as CodeAddressType,
            line_column.0 as i32);
        self.code.push(Rc::new(instr));
    }

    // todo: merge with parse_JNZ
    fn parse_JN(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let register = self.parse_register(&inner_pairs.next().unwrap());

        let label = String::from(inner_pairs.next().unwrap().as_str());

        let code_offset_option = self.labels.get(&label);
        if code_offset_option.is_none() {
            panic!("Unknown label '{}' at {}:{}", label, line_column.0, line_column.1);
        }

        let offset = code_offset_option.unwrap();
        let instr = create_JZ(
            register,
            *offset as CodeAddressType,
            line_column.0 as i32);
        self.code.push(Rc::new(instr));
    }

    fn parse_data(&mut self, pair: Pair<Rule>) {
        let mut inner_pairs = pair.into_inner();
        let var_pair = inner_pairs.next().unwrap();
        let line_column = self.get_line_column(&var_pair);
        let value_pair = inner_pairs.next().unwrap();

        let variable_name = String::from(var_pair.as_str());
        if !is_valid_variable_name(&variable_name) {
            panic!("Illegal variable name '{}' at {}:{}", variable_name, line_column.0, line_column.1);
        }

        let value: i32 = self.parse_integer(&value_pair);
        if self.data_section.contains_key(&variable_name) {
            panic!("Duplicate variable declaration '{}' at {}:{}", variable_name, line_column.0, line_column.1);
        }
        self.data_section.insert(variable_name.clone(), Rc::new(Data { value, offset: self.heap_size }));
        self.heap_size += 1;
    }


    fn get_line_column(&mut self, pair: &Pair<Rule>) -> (usize, usize) {
        let start_pos = pair.as_span().start_pos();
        let (line, column) = start_pos.line_col();
        (line, column)
    }

    fn parse_integer(&mut self, pair: &Pair<Rule>) -> i32 {
        pair.as_str().trim().parse().unwrap()
    }

    fn parse_register(&mut self, pair: &Pair<Rule>) -> u16 {
        let s = pair.as_str();
        let register = s[1..].parse().unwrap();
        // if register>=self.cpu_config.phys_reg_count{
        //
        // }
        return register;
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
pub(crate) fn load(cpu_config: CPUConfig, path: &str) -> Program {
    let mut loader = Loader {
        heap_size: 0,
        cpu_config,
        path: String::from(path),
        code: Vec::<Rc<Instr>>::new(),
        data_section: HashMap::<String, Rc<Data>>::new(),
        labels: HashMap::<String, usize>::new(),
    };

    return loader.load();
}
