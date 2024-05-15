use std::rc::Rc;
use std::cell::RefCell;
use crate::cpu::CPUConfig;
use crate::instructions::instructions::{InstrQueue, is_control, Opcode, Program};

pub(crate) struct FrontendControl {
    pub(crate) ip_next_fetch: i64,
    pub(crate) halted: bool,
}

pub(crate) struct Frontend {
    instr_queue: Rc<RefCell<InstrQueue>>,
    n_wide: u8,
    frontend_control: Rc<RefCell<FrontendControl>>,
    program_option: Option<Rc<Program>>,
    trace: bool,
    exit: bool,
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
            frontend_control,
            exit: false,
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
                if self.frontend_control.borrow_mut().halted {
                    return;
                }

                let mut instr_queue = self.instr_queue.borrow_mut();
                let mut frontend_control = self.frontend_control.borrow_mut();
                for _ in 0..self.n_wide {
                    if self.exit {
                        return;
                    }

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

                    let instr = program.get_instr(frontend_control.ip_next_fetch as usize);
                    if self.trace {
                        println!("Frontend: ip_next_fetch: {} decoded {}", frontend_control.ip_next_fetch, instr);
                    }
                    let opcode = instr.opcode;
                    if opcode == Opcode::EXIT {
                        self.exit = true;
                    }

                    // todo: what about cloning?
                    instr_queue.enqueue(instr);

                    if is_control(opcode) {
                        frontend_control.halted = true;
                        return;
                    }
                    frontend_control.ip_next_fetch += 1;
                }
            }
        }
    }
}
