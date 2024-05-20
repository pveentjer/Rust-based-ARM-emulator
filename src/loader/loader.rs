use std::any::type_name;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use regex::Regex;
use Operand::{Register, Unused};

use crate::cpu::{SP, CPUConfig, GENERAL_ARG_REG_CNT, PC, LR, FP};
use crate::instructions::instructions::{Data, get_opcode, get_register, Instr, Opcode, Operand, Program, WordType};
use crate::instructions::instructions::Operand::Code;

#[derive(Parser)]
#[grammar = "loader/assembly.pest"]
struct AssemblyParser;

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

impl Loader {
    fn load(&mut self) {
        let path = &self.path;
        let input = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                panic!("Error reading file: {}", err);
            }
        };

        // first pass
        match AssemblyParser::parse(Rule::file, &input) {
            Ok(parsed) => {
                self.first_pass(parsed);
            }
            Err(err) => {
                panic!("Parsing error: {}", err);
            }
        };

        // second pass
        match AssemblyParser::parse(Rule::file, &input) {
            Ok(parsed) => {
                self.second_pass(parsed);
            }
            Err(err) => {
                panic!("Parsing error: {}", err);
            }
        };

        self.fix_control_flag();
    }

    fn first_pass(&mut self, root: Pairs<Rule>) {
        for pair in root {
            match pair.as_rule() {
                Rule::label => self.parse_label(pair),
                Rule::data_line => self.parse_data(pair),
                Rule::instr => { self.instr_cnt += 1 }
                _ => {}
            }
        }
    }

    fn second_pass(&mut self, root: Pairs<Rule>) {
        for pair in root {
            match pair.as_rule() {
                Rule::label => {}
                Rule::data_line => {}
                Rule::directive_line => self.parse_directive(pair),
                Rule::instr => self.parse_instr(pair),
                _ => {}
            }
        }
    }

    fn fix_control_flag(&mut self) {
        for instr in &mut self.code {
            if !instr.is_control {
                instr.is_control = Self::is_control(instr);
            }
        }
    }

    fn is_control(instr: &Instr) -> bool {
        instr.source.iter().any(|op| Self::is_control_operand(op)) ||
            instr.sink.iter().any(|op| Self::is_control_operand(op))
    }

    fn is_control_operand(op: &Operand) -> bool {
        matches!(op, Register(register) if *register == PC)
    }

    fn parse_directive(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();
        let directive_name = inner_pairs.next().unwrap().as_str();

        println!("Directive name: {}",directive_name);

        match directive_name {
            ".global" => {
                let target = self.parse_label_ref(&inner_pairs.next().unwrap());
                self.entry_point = target;
            }
            ".data" => {
            }
            ".text" =>{
            }
            _ => panic!("Unknown directive '{}'at  [{},{}] ",directive_name,line_column.0,line_column.1)
        }
    }

    fn parse_label(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let label = String::from(inner_pairs.next().unwrap().as_str());

        if self.labels.contains_key(&label) {
            panic!("Duplicate label '{}' at [{}:{}]", label, line_column.0, line_column.1);
        } else {
            self.labels.insert(label, self.instr_cnt);
        }
    }

    fn parse_instr(&mut self, pair: Pair<Rule>) {
        let line_column = self.get_line_column(&pair);
        let mut inner_pairs = pair.into_inner();

        let mnemonic = inner_pairs.next().unwrap().as_str();
        let opcode = get_opcode(mnemonic);

        if opcode.is_none() {
            panic!("Unknown mneumonic '{}' at [{}:{}]", mnemonic, line_column.0, line_column.1);

        }

        let mut operands = Vec::new();
        for operand_pair in inner_pairs {
            operands.push(self.parse_operand(&operand_pair));
        }

        let instr = self.create_instr(opcode.unwrap(), operands, line_column.0 as i32);
        self.code.push(instr);
    }

    fn create_instr(&self, opcode: Opcode, operands: Vec<Operand>, line: i32) -> Instr {
        let mut instr = Instr {
            cycles: 1,
            opcode,
            source_cnt: 0,
            source: [Unused, Unused, Unused],
            sink_cnt: 0,
            sink: [Unused, Unused],
            line,
            mem_stores: 0,
            is_control: false,
        };

        for (i, operand) in operands.iter().enumerate() {
            if i < 3 {
                instr.source[i] = operand.clone();
                instr.source_cnt += 1;
            } else if i - 3 < 2 {
                instr.sink[i - 3] = operand.clone();
                instr.sink_cnt += 1;
            }
        }

        instr
    }

    fn parse_operand(&self, pair: &Pair<Rule>) -> Operand {
        let line_column = self.get_line_column(&pair);
        let s = pair.as_str().to_lowercase();
        match pair.as_rule() {
            Rule::register => Register(self.parse_register(pair)),
            Rule::immediate => Operand::Immediate(self.parse_immediate(pair)),
            Rule::memory_access => Operand::Memory(self.parse_memory_access(pair)),
            Rule::variable_address => Operand::Memory(self.parse_variable_address(pair)),
            Rule::label_name => Code(self.parse_label_ref(pair) as WordType),
            _ => panic!("Unknown operand encountered {} at  at [{}:{}]",s,line_column.0,line_column.1),
        }
    }

    fn parse_register(&self, pair: &Pair<Rule>) -> u16 {
        let line_column = self.get_line_column(&pair);
        let name = pair.as_str();
        match get_register(name){
            None =>  panic!("Illegal register '{}' at [{}:{}]", name, line_column.0, line_column.1),
            Some(reg) => reg,
        }
    }

    fn parse_immediate(&self, pair: &Pair<Rule>) -> WordType {
        pair.as_str()[1..].parse().unwrap()
    }

    fn parse_memory_access(&self, pair: &Pair<Rule>) -> WordType {
        let inner_pairs = pair.clone().into_inner();
        let base_register = self.parse_register(&inner_pairs.clone().next().unwrap());

        if inner_pairs.clone().count() > 1 {
            let offset_pair = inner_pairs.clone().nth(1).unwrap();
            let offset = match offset_pair.as_rule() {
                Rule::register => self.parse_register(&offset_pair) as i64,
                Rule::immediate => self.parse_immediate(&offset_pair) as i64,
                _ => panic!("Unknown memory access offset"),
            };
            (base_register as i64 + offset) as WordType
        } else {
            base_register as WordType
        }
    }

    fn parse_variable_address(&self, pair: &Pair<Rule>) -> WordType {
        let variable_name = pair.as_str()[1..].to_string();
        if let Some(data) = self.data_section.get(&variable_name) {
            data.offset as WordType
        } else {
            panic!("Unknown variable '{}'", variable_name);
        }
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
        self.data_section.insert(variable_name.clone(), Rc::new(Data { value, offset: self.heap_size as u64 }));
        self.heap_size += 1;
    }

    fn get_line_column(&self, pair: &Pair<Rule>) -> (usize, usize) {
        let start_pos = pair.as_span().start_pos();
        start_pos.line_col()
    }

    fn parse_integer(&self, pair: &Pair<Rule>) -> i64 {
        pair.as_str().trim().parse().unwrap()
    }

    fn parse_label_ref(&self, pair: &Pair<Rule>) -> usize {
        let line_column = self.get_line_column(&pair);
        let label = String::from(pair.as_str());

        match self.labels.get(&label) {
            Some(code_address) => *code_address,
            None => {
                panic!("Unknown label '{}' at [{}:{}]", label, line_column.0, line_column.1)
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
pub fn load(cpu_config: CPUConfig, path: &str) -> Program {
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

    loader.load();

    let mut code = Vec::with_capacity(loader.code.len());
    for instr in loader.code {
        code.push(Rc::new(instr));
    }
    Program { code, data_items: loader.data_section.clone(), entry_point: loader.entry_point }
}
