use std::fs;
use std::rc::Rc;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use crate::instructions::{create_ADD, create_INC, create_LOAD, create_NOP, create_PRINTR, create_STORE, Instr, Program};


#[derive(Parser)]
#[grammar = "assembly.pest"]
struct AssemblyParser;


// for the time being we always return the same program
pub(crate) fn load(path: &str) -> Program {
    let mut code = Vec::<Rc<Instr>>::new();
    let mut line = 0;

    // Read the assembly code from a file (replace "input.asm" with your file path)
    let input = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            panic!("Error reading file: {}", err);
        }
    };
    match AssemblyParser::parse(Rule::assembly, &input) {
        Ok(parsed) => {
            println!("parsed length {}",parsed.len());
            for pair in parsed {
                println!("Pair: {}",pair.as_str());
                match pair.as_rule() {
                    Rule::data_section => println!("Found data section"),
                    Rule::data => println!("Found data "),
                    Rule::label => println!("Found label "),
                    Rule::instr_INC => println!("Found inc "),
                    Rule::instr_DEC => println!("Found dec "),
                    Rule::instr_LOAD => println!("Found load "),

                    // Rule::data => println!("Found data"),
                   // Rule::code_section => println!("Found code section"),
                   // Rule::instruction => println!("Found instruction section"),
                   // Rule::WHITESPACE | Rule::COMMENT | Rule::NEWLINE => println!("Whitespace/comment"),
                    _ => unreachable!()
                }
            }
        }
        Err(err) => {
            panic!("Parsing error: {}", err);
          //  eprintln!("Parsing error: {}", err);
        }
    }
    return Program::new(code);
}


// fn parse_data(pair: Pair<Rule>) {
//     for inner_pair in pair.into_inner() {
//         println!("parse_data {}",inner_pair.as_str());
//         match inner_pair.as_rule() {
//             Rule::identifier => println!("Identifier: {}", inner_pair.as_str()),
//             Rule::integer => println!("Integer: {}", inner_pair.as_str()),
//             _ => unreachable!(),
//         }
//     }
// }
//
// fn parse_code(pair: Pair<Rule>) {
//     for inner_pair in pair.into_inner() {
//         match inner_pair.as_rule() {
//             Rule::label => parse_label(inner_pair),
//             Rule::load => println!("Load Instruction: {}", inner_pair.as_str()),
//             Rule::printr => println!("Printr Instruction: {}", inner_pair.as_str()),
//             Rule::dec => println!("Dec Instruction: {}", inner_pair.as_str()),
//             Rule::jnz => println!("Jnz Instruction: {}", inner_pair.as_str()),
//             Rule::add => println!("Add Instruction: {}", inner_pair.as_str()),
//             Rule::store => println!("Store Instruction: {}", inner_pair.as_str()),
//             Rule::halt => println!("Halt Instruction: {}", inner_pair.as_str()),
//             _ => unreachable!(),
//         }
//     }
// }
//
// fn parse_label(pair: Pair<Rule>) {
//     println!("Label: {}", pair.as_str());
// }