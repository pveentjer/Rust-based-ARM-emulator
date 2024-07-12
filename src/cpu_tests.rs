use std::rc::Rc;

use crate::cpu::{CPU, CPUConfig};
use crate::instructions::instructions::{DWordType, Program, RegisterType};

#[cfg(test)]
mod tests {
    use crate::loader::loader::{load_from_string, LoadError};

    use super::*;

    #[test]
    fn test_same_src_dst_reg() {
        let src = r#"
.text
    MOV r0, #5;
    ADD r0, r0, #10;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(0, 15);
    }

    #[test]
    fn test_ADD() {
        let src = r#"
.text
    MOV r0, #100;
    MOV r1, #10;
    ADD r2, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(0, 100);
        harness.assert_reg_value(1, 10);
        harness.assert_reg_value(2, 110);
    }

    #[test]
    fn test_NEG() {
        let src = r#"
.text
    MOV r0, #100;
    NEG r1, r0;
    MOV r0, #0;
    NEG r2, r0;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        // todo: ugly constant (2^64-100)
        harness.assert_reg_value(1, 18446744073709551516);
        harness.assert_reg_value(2, 0);
    }

    #[test]
    fn test_AND() {
        let src = r#"
.text
    MOV r0, #0;
    MOV r1, #0;
    AND r2, r0, r1;
    MOV r0, #1;
    MOV r1, #0;
    AND r3, r0, r1;
    MOV r0, #0;
    MOV r1, #1;
    AND r4, r0, r1;
    MOV r0, #1;
    MOV r1, #1;
    AND r5, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(2, 0);
        harness.assert_reg_value(3, 0);
        harness.assert_reg_value(4, 0);
        harness.assert_reg_value(5, 1);
    }

    #[test]
    fn test_ORR() {
        let src = r#"
.text
    MOV r0, #0;
    MOV r1, #0;
    ORR r2, r0, r1;
    MOV r0, #1;
    MOV r1, #0;
    ORR r3, r0, r1;
    MOV r0, #0;
    MOV r1, #1;
    ORR r4, r0, r1;
    MOV r0, #1;
    MOV r1, #1;
    ORR r5, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(2, 0);
        harness.assert_reg_value(3, 1);
        harness.assert_reg_value(4, 1);
        harness.assert_reg_value(5, 1);
    }

    #[test]
    fn test_EOR() {
        let src = r#"
.text
    MOV r0, #0;
    MOV r1, #0;
    EOR r2, r0, r1;
    MOV r0, #1;
    MOV r1, #0;
    EOR r3, r0, r1;
    MOV r0, #0;
    MOV r1, #1;
    EOR r4, r0, r1;
    MOV r0, #1;
    MOV r1, #1;
    EOR r5, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(2, 0);
        harness.assert_reg_value(3, 1);
        harness.assert_reg_value(4, 1);
        harness.assert_reg_value(5, 0);
    }

    #[test]
    fn test_MVN() {
        let src = r#"
 .text
     MOV r0, #100;
     MVN r1, r0;
     MOV r0, #0;
     MVN r2, r0;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(1, 18446744073709551515); // NOT 100 = 2^64 - 101
        harness.assert_reg_value(2, 18446744073709551615); // NOT 0 = 2^64 - 1
    }

    #[test]
    fn test_SUB() {
        let src = r#"
.text
    MOV r0, #100;
    MOV r1, #10;
    SUB r2, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.load_program(src);
        harness.run(src);
        harness.assert_reg_value(0, 100);
        harness.assert_reg_value(1, 10);
        harness.assert_reg_value(2, 90);
    }

    #[test]
    fn test_RSB() {
        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #100;
    RSB r2, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.load_program(src);
        harness.run(src);
        harness.assert_reg_value(2, 90);
    }

    #[test]
    fn test_MUL() {
        let src = r#"
.text
    MOV r0, #100;
    MOV r1, #10;
    MUL r2, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(0, 100);
        harness.assert_reg_value(1, 10);
        harness.assert_reg_value(2, 1000);
    }

    #[test]
    fn test_loop_CMP_BNE() {
        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #0;
loop:
    SUB r0, r0, #1;
    PRINTR r0;
    ADD r1, r1, #1;
    CMP r0, #0;
    BNE loop;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 0);
        harness.assert_reg_value(1, 10);
    }

    #[test]
    fn test_loop_CMP_BGT() {
        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #0;
loop:
    SUB r0, r0, #1;
    ADD r1, r1, #1;
    CMP r0, #1;
    BGT loop;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 1);
        harness.assert_reg_value(1, 9);
    }

    #[test]
    fn test_loop_CMP_BGE() {
        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #0;
loop:
    SUB r0, r0, #1;
    ADD r1, r1, #1;
    CMP r0, #1;
    BGE loop;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 0);
        harness.assert_reg_value(1, 10);
    }

    #[test]
    fn test_loop_CMP_BLE() {
        let src = r#"
.text
loop:
    ADD r0, r0, #1;
    CMP r0, #10;
    BLE loop;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 11);
    }

    #[test]
    fn test_loop_CMP_BLT() {
        let src = r#"
.text
loop:
    ADD r0, r0, #1;
    CMP r0, #10;
    BLT loop;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 10);
    }

    #[test]
    fn test_loop_CMP_BEQ() {
        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #0;
loop:
    SUB r0, r0, #1;
    ADD r1, r1, #1;
    CMP r0, #0;
    BEQ end;
    B loop;
end:
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 0);
        harness.assert_reg_value(1, 10);
    }

    #[test]
    fn test_TEQ() {
        let src = r#"
.text
    MOV r0, #5;
    MOV r1, #5;
    TEQ r0, r1;
    BNE fail;
    TEQ r0, #0;
    BEQ fail;
    B end;
fail:
    MOV r2, #1;
end:
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(2, 0); // Ensure fail indicator is not set
    }

    #[test]
    fn test_TST() {
        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #12;
    TST r0, r1;
    BEQ fail;
    TST r0, #5;
    BNE fail;
    B end;
fail:
    MOV r2, #1;
end:
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(2, 0); // Ensure fail indicator is not set
    }

    #[test]
    fn test_LDR_STR() {
        let src = r#"
.data
    var_a: .dword 5
    var_b: .dword 10
    var_c: .dword 0
.text
    MOV r0, =var_a;
    LDR r0, [r0];
    MOV r1, =var_b;
    LDR r1, [r1];
    ADD r2, r0, r1;
    MOV r0, =var_c;
    STR r2, [r0];
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_variable_value("var_c", 15);
    }

    #[test]
    fn test_LDR() {
        let src = r#"
.data
    var_a: .dword 5
.text
    MOV r0, =var_a;
    LDR r0, [r0];
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 5);
    }

    #[test]
    fn test_STR() {
        let src = r#"
.data
    var_a: .dword 0
.text
    MOV r0, =var_a;
    MOV r1, #10;
    STR r1, [r0];
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_variable_value("var_a", 10);
    }

    // Ensures that stores update to memory out of order even they can be performed in order.
    #[test]
    fn test_STR_WAW() {
        let src = r#"
.data
    var_a: .dword 0
.text
    mov r0, =var_a;
    mov r1, #1;
    str r1, [r0];
    mov r2, #2;
    str r2, [r0];
    mov r3, #3;
    str r3, [r0];
    mov r4, #4;
    str r4, [r0];
    mov r5, #5;
    str r5, [r0];
    mov r6, #6;
    str r6, [r0];
    mov r7, #7;
    str r7, [r0];
    mov r8, #8;
    str r8, [r0];
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_variable_value("var_a", 8);
    }

    #[test]
    fn test_STR_loop() {
        let src = r#"
.data
    var_a: .dword 0
.text
    MOV r0, #100;
    MOV r1, =var_a;
    MOV r2, #0;
loop:
    ADD r2, r2, #1;
    SUB r0, r0, #1;
    STR r2, [r1];
    CBNZ r0, loop;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_variable_value("var_a", 100);
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

        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 8);
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

        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(8, 1);
    }

    #[test]
    fn test_BL_RET() {
        let src = r#"
.global _start
.text
_add_numbers:
    ADD r2, r0, r1;
    RET;
_start:
    MOV r0, #5;
    MOV r1, #10;
    BL _add_numbers;
    ADD r2, r2, #1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(2, 16);
    }

    #[test]
    fn test_loop_CBZ() {
        let src = r#"
.text
    MOV r0, #10;
_loop:
    SUB r0, r0, #1;
    ADD r1, r1, #1;
    CBZ r0, _end;
    B _loop;
_end:
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(1, 10);
    }

    #[test]
    fn test_loop_CBNZ() {
        let src = r#"
.text
    MOV r0, #10;
    MOV r1, #20;
loop:
    SUB r0, r0, #1;
    ADD r1, r1, #1;
    CBNZ r0, loop;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(0, 0);
        harness.assert_reg_value(1, 30);
    }

    #[test]
    fn test_nested_loop_CBNZ() {
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
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(2, 100);
    }

    #[test]
    fn test_BL_BX() {
        let src = r#"
.global _start
.text
_add_numbers:
    ADD r2, r0, r1;
    BX lr;
_start:
    MOV r0, #5;
    MOV r1, #10;
    BL _add_numbers;
    ADD r2, r2, #1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);

        harness.assert_reg_value(2, 16);
    }

    #[test]
    fn test_binary_value() {
        let src = r#"
.text
    MOV r0, #0b1010;
    MOV r1, #0b1100;
    ADD r2, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(0, 10);
        harness.assert_reg_value(1, 12);
        harness.assert_reg_value(2, 22);
    }

    #[test]
    fn test_hexadecimal_value() {
        let src = r#"
.text
    MOV r0, #0xA;
    MOV r1, #0xC;
    ADD r2, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(0, 10);
        harness.assert_reg_value(1, 12);
        harness.assert_reg_value(2, 22);
    }

    #[test]
    fn test_octal_values() {
        let src = r#"
.text
    MOV r0, #0o10;
    MOV r1, #0o20;
    ADD r2, r0, r1;
"#;
        let mut harness = TestHarness::default();
        harness.run(src);
        harness.assert_reg_value(0, 8);   // Check r0 for 8
        harness.assert_reg_value(1, 16);  // Check r1 for 16
        harness.assert_reg_value(2, 24);  // Check r2 for 24 (8 + 16)
    }

    struct TestHarness {
        program: Option<Rc<Program>>,
        cpu: Option<CPU>,
        cpu_config: CPUConfig,
    }

    impl TestHarness {
        fn default() -> TestHarness {
            let cpu_config = Self::new_test_cpu_config();
            TestHarness {
                program: None,
                cpu: Some(CPU::new(&cpu_config.clone())),
                cpu_config: cpu_config,
            }
        }

        fn new_test_cpu_config() -> CPUConfig {
            let mut cpu_config = CPUConfig::default();
            cpu_config.frequency_hz = 1000;
            cpu_config
        }

        fn run(&mut self, src: &str) {
            self.program = Some(self.load_program(src));
            let program = Rc::clone(self.program.as_ref().unwrap());
            self.cpu.as_mut().unwrap().run(&program);
        }

        fn load_program(&mut self, src: &str) -> Rc<Program> {
            let load_result = load_from_string(self.cpu_config.clone(), src.to_string());
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

        fn assert_reg_value(&self, reg: RegisterType, value: DWordType) {
            if let Some(ref cpu) = self.cpu {
                let reg_file = cpu.arch_reg_file.borrow();
                assert_eq!(reg_file.get_value(reg), value);
            } else {
                panic!("CPU is not initialized");
            }
        }

        fn assert_variable_value(&self, name: &str, value: DWordType) {
            if let Some(ref cpu) = self.cpu {
                let program = self.program.as_ref().expect("Program not initialized");
                let data_item = program.data_items.get(name).expect("Data item not found");
                let offset = data_item.offset;
                let memory_subsystem = cpu.memory_subsystem.borrow();
                match memory_subsystem.memory.get(offset as usize) {
                    Some(&actual_value) => {
                        assert_eq!(actual_value, value, "Variable '{}' does not have the expected value", name);
                    }
                    None => {
                        panic!("Memory offset {} is invalid", offset);
                    }
                }
            } else {
                panic!("CPU is not initialized");
            }
        }
    }
}
