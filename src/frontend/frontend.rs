use std::cell::RefCell;
use std::rc::Rc;

use crate::cpu::{ArgRegFile, CPUConfig, PC, PerfCounters, Trace};
use crate::instructions::instructions::{EXIT, Instr, InstrQueue, Opcode, Program, WordType};

pub(crate) struct FrontendControl {
    pub(crate) halted: bool,
}

pub(crate) struct Frontend {
    instr_queue: Rc<RefCell<InstrQueue>>,
    n_wide: u8,
    frontend_control: Rc<RefCell<FrontendControl>>,
    program_option: Option<Rc<Program>>,
    trace: Trace,
    exit: bool,
    perf_counters: Rc<RefCell<PerfCounters>>,
    arch_reg_file: Rc<RefCell<ArgRegFile>>,
}

impl Frontend {
    pub(crate) fn new(cpu_config: &CPUConfig,
                      instr_queue: Rc<RefCell<InstrQueue>>,
                      frontend_control: Rc<RefCell<FrontendControl>>,
                      perf_counters: Rc<RefCell<PerfCounters>>,
                      arch_reg_file: Rc<RefCell<ArgRegFile>>,
    ) -> Frontend {
        Frontend {
            instr_queue,
            n_wide: cpu_config.frontend_n_wide,
            program_option: None,
            trace: cpu_config.trace.clone(),
            frontend_control,
            exit: false,
            perf_counters,
            arch_reg_file,
        }
    }

    pub(crate) fn init(&mut self, program: &Rc<Program>) {
        self.program_option = Some(Rc::clone(program));
        self.arch_reg_file.borrow_mut().set_value(PC, program.entry_point as WordType);
    }

    pub(crate) fn do_cycle(&mut self) {
        match &self.program_option {
            None => return,
            Some(program) => {
                let mut instr_queue = self.instr_queue.borrow_mut();
                let mut frontend_control = self.frontend_control.borrow_mut();
                let mut perf_counters = self.perf_counters.borrow_mut();
                let mut arch_reg_file = self.arch_reg_file.borrow_mut();

                if frontend_control.halted {
                    return;
                }

                for _ in 0..self.n_wide {
                    if self.exit {
                        return;
                    }

                    if instr_queue.is_full() {
                        break;
                    }

                    // todo: we still need mechanism to 'stall' the pipeline. E.g

                    // MOV IP, 10
                    // B foobar

                    let pc_value = arch_reg_file.get_value(PC) as usize;
                    let instr = if program.code.len() == pc_value {
                        // at the end of the program
                        Rc::new(EXIT)
                    } else {
                        program.get_instr(pc_value)
                    };

                    if self.trace.decode {
                        println!("Frontend: pc: {}  '{}'", pc_value, instr);
                    }

                    if instr.opcode == Opcode::EXIT {
                        self.exit = true;
                    }

                    let tail_index = instr_queue.tail_index();
                    let mut slot = instr_queue.get_mut(tail_index);

                    let pc_value_next = if instr.is_branch() {
                        slot.branch_target_predicted = Self::predict(pc_value, &instr);
                        println!("Frontend branch predicted={}", slot.branch_target_predicted);
                        slot.branch_target_predicted
                    } else {
                        pc_value + 1
                    };
                    arch_reg_file.set_value(PC, pc_value_next as WordType);

                    slot.instr = instr;
                    instr_queue.tail_bump();
                    perf_counters.decode_cnt += 1;
                }
            }
        }
    }

    // A static branch predictor that will speculate that backwards branches are always taken
    fn predict(ip: usize, instr: &Instr) -> usize {
        let branch_target = match instr.opcode {
            Opcode::B => {
                // this is an unconditional branch. So we can predict with 100% certainty
                return instr.source[0].get_code_address() as usize;
            }
            Opcode::BX => 0,
            Opcode::BL => 0,
            Opcode::CBNZ |
            Opcode::CBZ => instr.source[1].get_code_address() as usize,
            Opcode::BNE |
            Opcode::BLE |
            Opcode::BLT |
            Opcode::BGE |
            Opcode::BGT |
            Opcode::BEQ => instr.source[0].get_code_address() as usize,
            _ => unreachable!(),
        };

        if branch_target < ip {
            // backwards branches are always taken
            branch_target
        } else {
            ip + 1
        }
    }
}
