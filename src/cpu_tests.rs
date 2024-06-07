use std::rc::Rc;
use crate::cpu::{CPU, CPUConfig};
use crate::instructions::instructions::{Program, DWordType};

// This is super ugly; the test will be moved to its own directory
#[cfg(test)]
mod tests {
    use crate::loader::loader::{load_from_string, LoadError};
    use super::*;

    #[test]
    fn test_add() {
        let cpu_config = new_test_cpu_config();
        let src = r#"
.text
    MOV r0, #100;
    MOV r1, #10;
    ADD r2, r0, r1;
"#;
        let program = load_program(&cpu_config, src);
        let mut cpu = CPU::new(&cpu_config);
        cpu.run(&program);

        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(0), 100 as DWordType);
        assert_eq!(reg_file.get_value(1), 10 as DWordType);
        assert_eq!(reg_file.get_value(2), 110 as DWordType);
    }


    #[test]
    fn test_sub() {
        let cpu_config = new_test_cpu_config();
        let src = r#"
.text
    MOV r0, #100;
    MOV r1, #10;
    SUB r2, r0, r1;
"#;
        let program = load_program(&cpu_config, src);
        let mut cpu = CPU::new(&cpu_config);
        cpu.run(&program);

        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(0), 100 as DWordType);
        assert_eq!(reg_file.get_value(1), 10 as DWordType);
        assert_eq!(reg_file.get_value(2), 90 as DWordType);
    }

    #[test]
    fn test_mul() {
        let cpu_config = new_test_cpu_config();
        let src = r#"
.text
    MOV r0, #100;
    MOV r1, #10;
    MUL r2, r0, r1;
"#;
        let program = load_program(&cpu_config, src);
        let mut cpu = CPU::new(&cpu_config);
        cpu.run(&program);

        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(0), 100 as DWordType);
        assert_eq!(reg_file.get_value(1), 10 as DWordType);
        assert_eq!(reg_file.get_value(2), 1000 as DWordType);
    }

    #[test]
    fn test_loop() {
        let cpu_config = new_test_cpu_config();

        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #20;
loop:
    SUB r0, r0, #1;
    ADD r1, r1, #1;
    CBNZ r0, loop;
"#;
        let program = load_program(&cpu_config, src);
        let mut cpu = CPU::new(&cpu_config);
        cpu.run(&program);

        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(0), 0 as DWordType);
        assert_eq!(reg_file.get_value(1), 30 as DWordType);
    }


//     #[test]
//     fn test_load_store() {
//         let src = r#"
// .data
//     var_a: .dword 5
//     var_b: .dword 10
//     var_c: .dword 0
// .text
//     MOV r0, =var_a;
//     LDR r0, r0;
//     MOV r1, =var_b;
//     LDR r1, r1;
//     ADD r2, r0, r1;
//     MOV r0, =var_c;
//     STR r2, r0;
// "#;
//         let cpu_config = new_test_cpu_config();
//         let program = load_program(&cpu_config, src);
//         let mut cpu = CPU::new(&cpu_config);
//         cpu.run(&program);
//         let reg_file = cpu.arch_reg_file.borrow();
//         assert_eq!(reg_file.get_value(0), 5 as WordType);
//         assert_eq!(reg_file.get_value(1), 10 as WordType);
//         assert_eq!(reg_file.get_value(2), 15 as WordType);
//     }

    #[test]
    fn test_load() {
        let src = r#"
.data
    var_a: .dword 5
.text
    MOV r0, =var_a;
    LDR r0, r0;
"#;
        let cpu_config = new_test_cpu_config();
        let program = load_program(&cpu_config, src);
        let mut cpu = CPU::new(&cpu_config);
        cpu.run(&program);
        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(0), 5 as DWordType);
    }

    #[test]
    fn test_waw() {
        let src = r#"
.text
    MOV r0, #1;
    MOV r0, #2;
    MOV r0, #3;
    MOV r0, #4;
    MOV r0, #5;
    MOV r0, #6;
    MOV r0, #7;
    MOV r0, #8;
"#;

        let cpu = run(src);
        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(0), 8 as DWordType);
    }

    #[test]
    fn test_dependency_chain() {
        let src = r#"
.text
    MOV r0, #1;
    MOV r1, r0;
    MOV r2, r1;
    MOV r3, r2;
    MOV r4, r3;
    MOV r5, r4;
    MOV r6, r5;
    MOV r7, r6;
    MOV r8, r7;
"#;

        let cpu = run(src);
        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(8), 1 as DWordType);
    }

    fn run(src: &str) -> CPU {
        let cpu_config = new_test_cpu_config();


        let program = load_program(&cpu_config, src);
        let mut cpu = CPU::new(&cpu_config);
        cpu.run(&program);
        cpu
    }


    #[test]
    fn test_nested_CBNZ() {
        let cpu_config = new_test_cpu_config();
        let src = r#"
.text
    MOV r0, #10;
_loop_outer:
    MOV r1, #10;
_loop_inner:
    SUB r1, r1, #1;
    ADD r2, r2, #1;
    CBNZ r1, _loop_inner;
    SUB r0, r0, #1;
    CBNZ r0, _loop_outer;
"#;
        let program = load_program(&cpu_config, src);
        let mut cpu = CPU::new(&cpu_config);
        cpu.run(&program);

        let reg_file = cpu.arch_reg_file.borrow();
        assert_eq!(reg_file.get_value(2), 100 as DWordType);
    }

    fn new_test_cpu_config() -> CPUConfig {
        let mut cpu_config = CPUConfig::default();
        cpu_config.frequency_hz = 1000;
        cpu_config
    }

    fn load_program(cpu_config: &CPUConfig, src: &str) -> Rc<Program> {
        let load_result = load_from_string(cpu_config.clone(), src.to_string());
        let program = match load_result {
            Ok(p) => Rc::new(p),
            Err(err) => {
                match err {
                    LoadError::ParseError(msg) => {
                        println!("{}", msg);
                        assert!(false);
                        unreachable!();
                    }

                    LoadError::AnalysisError(msg_vec) => {
                        for msg in msg_vec {
                            println!("{}", msg);
                        }
                        assert!(false);
                        unreachable!();
                    }
                    LoadError::NotFoundError(msg) => {
                        println!("{}", msg);
                        unreachable!();
                    }
                    LoadError::IOError(msg) => {
                        println!("{}", msg);
                        unreachable!();
                    }
                }
            }
        };
        program
    }
}
