use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use crate::cpu::CPUConfig;
use crate::instructions::{create_ADD, create_DEC, create_DIV, create_INC, create_LOAD, create_MOD, create_MUL, create_NOP, create_PRINTR, create_STORE, create_SUB, Data, Instr, MemoryType, Program, RegisterType, WordType};


#[derive(Parser)]
#[grammar = "assembly.pest"]
struct AssemblyParser;

pub(crate) struct Loader {}

impl Loader {
    pub fn new() -> Self {
        Self {}
    }
}


// for the time being we always return the same program
pub(crate) fn load(cpu_config: &CPUConfig, path: &str) -> Program {
    let mut code = Vec::<Rc<Instr>>::new();
    let mut data_section = HashMap::<String, Rc<Data>>::new();

    let input = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            panic!("Error reading file: {}", err);
        }
    };

    let mut heap_size: MemoryType = 0;

    match AssemblyParser::parse(Rule::file, &input) {
        Ok(parsed) => {
            for pair in parsed {
                match pair.as_rule() {
                    Rule::assembly => {}
                    Rule::file => {}
                    Rule::EOI => {}
                    Rule::data_section => println!("Found data section"),
                    Rule::data => {
                        let mut inner_pairs = pair.into_inner();
                        let var = inner_pairs.next().unwrap();
                        let name = String::from(var.as_str());
                        let value: i32 = parse_integer(&mut inner_pairs);

                        if data_section.contains_key(&name) {
                            let line_column = get_line_column(&var);
                            panic!("Duplicate variable declaration '{}' at {}:{}", name, line_column.0, line_column.1);
                        }
                        data_section.insert(name.clone(), Rc::new(Data { value, offset: heap_size }));
                        heap_size += 1;
                    }
                    Rule::label => println!("Found label "),
                    Rule::instr_INC => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_INC(
                            parse_register(&mut inner_pairs),
                            0)));
                    }
                    Rule::instr_DEC => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_DEC(
                            parse_register(&mut inner_pairs),
                            0)));
                    }
                    Rule::instr_NOP => code.push(Rc::new(create_NOP(0))),
                    Rule::instr_PRINTR => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_PRINTR(parse_register(&mut inner_pairs), 0)));
                    }
                    Rule::instr_LOAD => {
                        let mut inner_pairs = pair.into_inner();

                        let name = parse_variable(&mut inner_pairs);
                        let register = parse_register(&mut inner_pairs);

                        let data_option = data_section.get(&name);
                        if data_option.is_none() {
                            // todo: add line
                            panic!("Unknown variable '{}'", name);
                        }

                        let data = data_option.unwrap();
                        code.push(Rc::new(create_LOAD(data.offset, register as RegisterType, 0)));
                    }
                    Rule::instr_STORE => {
                        let mut inner_pairs = pair.into_inner();

                        let register = parse_register(&mut inner_pairs);
                        let name = parse_variable(&mut inner_pairs);

                        let data_option = data_section.get(&name);
                        if data_option.is_none() {
                            // todo: add line
                            panic!("Unknown variable '{}'", name);
                        }

                        let data = data_option.unwrap();
                        code.push(Rc::new(create_STORE(register as RegisterType, data.offset, 0)));
                    }
                    Rule::instr_ADD => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_ADD(
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            0)));
                    }
                    Rule::instr_SUB => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_SUB(
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            0)));
                    }
                    Rule::instr_MUL => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_MUL(
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            0)));
                    }
                    Rule::instr_DIV => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_DIV(
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            0)));
                    }
                    Rule::instr_MOD => {
                        let mut inner_pairs = pair.into_inner();
                        code.push(Rc::new(create_MOD(
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            parse_register(&mut inner_pairs),
                            0)));
                    }

                    // Rule::data => println!("Found data"),
                    // Rule::code_section => println!("Found code section"),
                    // Rule::instruction => println!("Found instruction section"),
                    // Rule::WHITESPACE | Rule::COMMENT | Rule::NEWLINE => println!("Whitespace/comment"),
                    _ => panic!("Unknown rule encountered: '{:?}'", pair.as_rule())
                }
            }
        }
        Err(err) => {
            panic!("Parsing error: {}", err);
            //  eprintln!("Parsing error: {}", err);
        }
    };
    return Program::new(code, data_section);
}

fn get_line_column(pair: &Pair<Rule>) -> (usize, usize) {
    let start_pos = pair.as_span().start_pos();
    let (line, column) = start_pos.line_col();
    (line, column)
}

fn parse_integer(inner_pairs: &mut Pairs<Rule>) -> i32 {
    inner_pairs.next().unwrap().as_str().trim().parse().unwrap()
}

fn parse_register(inner_pairs: &mut Pairs<Rule>) -> u16 {
    let s = inner_pairs.next().unwrap().as_str();
    return s[1..].parse().unwrap();
}

fn parse_variable(inner_pairs: &mut Pairs<Rule>) -> String {
    let s = String::from(inner_pairs.next().unwrap().as_str());
    let s_len = s.len();
    let x = &s[1..s_len - 1];
    return String::from(x);
}