use std::cell::RefCell;
use std::rc::Rc;

use crate::cpu::{ArgRegFile, CPUConfig, PC, PerfCounters, Trace};
use crate::instructions::instructions::{EXIT, InstrQueue, Opcode, Program, WordType};

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

                    let pc_value = arch_reg_file.get_value(PC) as usize;
                    let instr = if program.code.len() == pc_value {
                        // at the end of the program
                         Rc::new(EXIT)
                    }else{
                        program.get_instr(pc_value)
                    };

                    if self.trace.decode {
                        println!("Frontend: ip_next_fetch: {} decoded {}", pc_value, instr);
                    }

                    if instr.opcode == Opcode::EXIT {
                        self.exit = true;
                    }

                    let is_control = instr.is_control;

                    // todo: what about cloning?
                    instr_queue.enqueue(instr);

                    // move the PC to the next instruction.
                    arch_reg_file.set_value(PC, (pc_value + 1) as WordType);
                    perf_counters.decode_cnt += 1;

                    if is_control {
                        frontend_control.halted = true;
                        return;
                    }
                }
            }
        }
    }
}
