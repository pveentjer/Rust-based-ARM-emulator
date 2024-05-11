use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use crate::cpu::CPUConfig;
use crate::instructions::{create_ADD, create_DEC, create_INC, create_LOAD, create_NOP, create_PRINTR, create_STORE, Data, Instr, MemoryType, Program, RegisterType, WordType};


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
            println!("parsed length {}", parsed.len());
            for pair in parsed {
                println!("Pair: {}", pair.as_str());
                match pair.as_rule() {
                    Rule::assembly => {},
                    Rule::file => {},
                    Rule::EOI => {},
                    Rule::data_section => println!("Found data section"),
                    Rule::data => {
                        let mut inner_pairs = pair.into_inner();
                        let name = parse_variable(&mut inner_pairs);
                        let value: i32 = parse_integer(&mut inner_pairs);

                        if data_section.contains_key(&name) {
                            panic!("Duplicate variable declaration '{}'", name);
                        } else {
                            data_section.insert(name.clone(),Rc::new(Data {value, offset: heap_size }));
                            heap_size + 1;
                        }

                        println!("variable {}={}", name, value);
                        // parse the data section
                    }
                    Rule::label => println!("Found label "),
                    Rule::instr_INC => {
                        let mut inner_pairs = pair.into_inner();
                        let register = parse_integer(&mut inner_pairs);
                        // todo: validate register
                        code.push(Rc::new(create_INC(register as RegisterType, 0)));
                    },
                    Rule::instr_DEC => {
                        let mut inner_pairs = pair.into_inner();
                        let register = parse_integer(&mut inner_pairs);
                        // todo: validate register
                        code.push(Rc::new(create_DEC(register as RegisterType, 0)));
                    },
                    Rule::instr_NOP => code.push(Rc::new(create_NOP(0))),
                    Rule::instr_PRINTR => {
                        let mut inner_pairs = pair.into_inner();
                        let register = parse_integer(&mut inner_pairs);
                        // todo: validate register
                        code.push(Rc::new(create_PRINTR(register as RegisterType, 0)));
                    },
                    Rule::instr_LOAD => {
                        let mut inner_pairs = pair.into_inner();
                        println!("first {}",inner_pairs.peek().unwrap().as_str());

                        let name = parse_variable(&mut inner_pairs);
                        let register: i32 = parse_integer(&mut inner_pairs);

                        let data_option = data_section.get(&name);
                        if data_option.is_none(){
                            // todo: add line
                            panic!("Could not find variable declaration '{}'", name);
                        }

                        let data = data_option.unwrap();
                        code.push(Rc::new(create_LOAD(data.offset, register as RegisterType, 0)));
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

fn parse_integer(inner_pairs: &mut Pairs<Rule>) -> i32 {
    inner_pairs.next().unwrap().as_str().trim().parse().unwrap()
}

fn parse_variable(inner_pairs: &mut Pairs<Rule>) -> String {
    inner_pairs.next().unwrap().as_str().trim().to_string()
}