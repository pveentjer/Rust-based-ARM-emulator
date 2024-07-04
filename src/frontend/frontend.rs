use std::cell::RefCell;
use std::rc::Rc;

use crate::cpu::{ArgRegFile, CPUConfig, PC, PerfCounters, Trace};
use crate::instructions::instructions::{Branch, DWordType, EXIT, Instr, InstrQueue, Opcode, Program};

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
    pub(crate) fn new(
        cpu_config: &CPUConfig,
        instr_queue: &Rc<RefCell<InstrQueue>>,
        frontend_control: &Rc<RefCell<FrontendControl>>,
        perf_counters: &Rc<RefCell<PerfCounters>>,
        arch_reg_file: &Rc<RefCell<ArgRegFile>>,
    ) -> Frontend {
        Frontend {
            instr_queue: Rc::clone(instr_queue),
            n_wide: cpu_config.frontend_n_wide,
            program_option: None,
            trace: cpu_config.trace.clone(),
            frontend_control: Rc::clone(frontend_control),
            exit: false,
            perf_counters: Rc::clone(perf_counters),
            arch_reg_file: Rc::clone(arch_reg_file),
        }
    }

    pub(crate) fn init(&mut self, program: &Rc<Program>) {
        self.program_option = Some(Rc::clone(program));
        self.arch_reg_file.borrow_mut().set_value(PC, program.entry_point as DWordType);
    }

    pub(crate) fn do_cycle(&mut self) {
        match &self.program_option {
            None => return,
            Some(program) => {
                let mut instr_queue = self.instr_queue.borrow_mut();
                let frontend_control = self.frontend_control.borrow_mut();
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

                    let pc = arch_reg_file.get_value(PC) as usize;
                    let instr = if program.code.len() == pc {
                        // at the end of the program
                        Rc::new(EXIT)
                    } else {
                        program.get_instr(pc)
                    };

                    if self.trace.decode {
                        println!("Frontend: pc: {}  '{}'", pc, instr);
                    }

                    if let Instr::Synchronization { synchronization: fields } = instr.as_ref() {
                        if fields.opcode == Opcode::EXIT{
                            self.exit = true;
                        }
                    }

                    let tail_index = instr_queue.tail_index();
                    let slot = instr_queue.get_mut(tail_index);

                    let pc_value_next = match instr.as_ref() {
                        Instr::Branch { branch: fields } => {
                            slot.branch_target_predicted = Self::predict(pc, fields);
                            //println!("Frontend branch predicted={}", slot.branch_target_predicted);
                            slot.branch_target_predicted
                        }
                        _ => pc + 1,
                    };
                    arch_reg_file.set_value(PC, pc_value_next as DWordType);

                    slot.instr = instr;
                    slot.pc = pc;
                    instr_queue.tail_bump();
                    perf_counters.decode_cnt += 1;
                }
            }
        }
    }

    // A static branch predictor that will speculate that backwards branches are taken.
    // In the future better branch predictors can be added.
    fn predict(ip: usize, branch: &Branch) -> usize {
        let branch_target = match branch.opcode {
            Opcode::B |
            Opcode::BL => return branch.offset as usize, // unconditional branch can always be predicted accurately
            Opcode::RET => 0,
            Opcode::BX => 0,
            Opcode::CBNZ |
            Opcode::CBZ |
            Opcode::BNE |
            Opcode::BLE |
            Opcode::BLT |
            Opcode::BGE |
            Opcode::BGT |
            Opcode::BEQ => branch.offset,
            _ => unreachable!(),
        };

        if (branch_target as usize) < ip {
            // backwards branches are always taken
            branch_target as usize
        } else {
            ip + 1
        }
    }
}
