use std::rc::Rc;
use std::cell::RefCell;
use crate::cpu::CPUConfig;
use crate::instructions::{InstrQueue, is_control, Program};

pub(crate) struct FrontendControl {
    // indicates that there is an instruction in the pipeline that could cause a control
    // hazard if they frontend would move to the next instruction.
    pub(crate) ip_next_fetch: i64,
    pub(crate) control_hazard: bool,
}

pub(crate) struct Frontend {
    instr_queue: Rc<RefCell<InstrQueue>>,
    n_wide: u8,
    frontend_control: Rc<RefCell<FrontendControl>>,
    program_option: Option<Rc<Program>>,
    trace: bool,
}

impl Frontend {
    pub(crate) fn new(cpu_config: &CPUConfig,
                      instr_queue: Rc<RefCell<InstrQueue>>,
                      frontend_control: Rc<RefCell<FrontendControl>>,
    ) -> Frontend {
        Frontend {
            instr_queue,
            n_wide: cpu_config.frontend_n_wide,
            program_option: None,
            trace: cpu_config.trace,
            frontend_control: frontend_control,
        }
    }

    pub(crate) fn init(&mut self, program: &Rc<Program>) {
        self.program_option = Some(Rc::clone(program));
        self.frontend_control.borrow_mut().ip_next_fetch = 0;
    }

    pub(crate) fn do_cycle(&mut self) {
        match &self.program_option {
            None => return,
            Some(program) => {
                if self.frontend_control.borrow_mut().control_hazard {
                    return;
                }

                let mut instr_queue = self.instr_queue.borrow_mut();
                let mut frontend_control = self.frontend_control.borrow_mut();
                for _ in 0..self.n_wide {
                    if instr_queue.is_full() {
                        break;
                    }

                    if program.code.len() == frontend_control.ip_next_fetch as usize {
                        // at the end of the program
                        return;
                    }

                    if frontend_control.ip_next_fetch == -1 {
                        break;
                    }

                    let instr = program.get_code(frontend_control.ip_next_fetch as usize);
                    if self.trace {
                        println!("Frontend: ip_next_fetch: {} decoded {}", frontend_control.ip_next_fetch, instr);
                    }

                    let opcode = instr.opcode;

                    instr_queue.enqueue(instr);

                    if is_control(opcode) {
                        frontend_control.control_hazard = true;
                        return;
                    }
                    frontend_control.ip_next_fetch += 1;
                }
            }
        }
    }
}
