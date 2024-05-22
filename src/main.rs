use std::fs;
use std::rc::Rc;
use lalrpop_util::{lalrpop_mod, ParseError};

use crate::cpu::{CPU, CPUConfig, Trace};
use crate::instructions::instructions::{Program, SourceLocation};
use crate::loader::loader::{load, LoadError};

mod cpu;
mod loader;
mod frontend;
mod backend;
mod instructions;
mod memory_subsystem;
mod ast;


lalrpop_mod!(pub assembly);

fn get_line_and_column(input: &str, offset: usize) -> SourceLocation {
    let mut line = 1;
    let mut col = 1;
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
    SourceLocation{line:line, column:col}
}

fn main() {
    let cpu_config = CPUConfig {
        phys_reg_count: 64,
        frontend_n_wide: 4,
        instr_queue_capacity: 8,
        frequency_hz: 4,
        rs_count: 16,
        memory_size: 32,
        sb_capacity: 16,
        lfb_count: 8,
        rob_capacity: 32,
        eu_count: 16,
        trace: Trace {
            decode: true,
            issue: true,
            dispatch: true,
            execute: true,
            retire: true,
            cycle: true,
        },
        retire_n_wide: 4,
        dispatch_n_wide: 4,
        issue_n_wide: 4,
        stack_capacity: 32,
    };

    let path = "foo.asm";
    println!("Loading {}",path);
    let load_result = load(cpu_config.clone(), path);
    let program = match load_result {
        Ok(p) => Rc::new(p),
        Err(err) => {
            match err {
                LoadError::ParseError(msg) =>  panic!("{}", msg),
            }
        },
    };

    let mut cpu = CPU::new(&cpu_config);
    cpu.run(&program);

    //
    //  let mut input = match fs::read_to_string(path) {
    //     Ok(content) => content,
    //     Err(err) => {
    //         panic!("Error reading file: {}", err);
    //     }
    // };

    // let input_str = input.as_str();
    //
    // let parse_result = assembly::AssemblyParser::new()
    //     .parse(input_str);
    //
    // match parse_result {
    //     Ok(_)=>{
    //         println!("Parse success");
    //     }
    //     Err(err)=>{
    //         match err {
    //             ParseError::InvalidToken { location } => {
    //                 let loc = get_line_and_column(input_str, location);
    //                 println!("Invalid token at  at {}:{}", loc.line, loc.column);
    //             }
    //             ParseError::UnrecognizedToken { token, expected } => {
    //                 let loc = get_line_and_column(input_str, token.0);
    //                 println!("Unrecognized token '{}' at {}:{}. Expected: {:?}", token.1, loc.line, loc.column, expected);
    //             }
    //             ParseError::ExtraToken { token } => {
    //                 let loc = get_line_and_column(input_str, token.0);
    //                 println!("Extra token '{}' at {}:{}", token.1, loc.line, loc.column);
    //             }
    //             _ => println!("Error: {:?}", err),
    //         }
    //         //
    //         // let loc = get_line_and_column(input.as_str(), e.)
    //         // panic!("{}",e);
    //     }
    // }
}
