use std::rc::Rc;
use std::cell::RefCell;
use crate::cpu::CPUConfig;
use crate::instructions::{InstrQueue, Program};

pub(crate) struct Frontend {
    instr_queue: Rc<RefCell<InstrQueue>>,
    n_wide: u8,
    ip_next_fetch: i64,
    program_option: Option<Program>,
}

impl Frontend {
    pub(crate) fn new(cpu_config: & CPUConfig, instr_queue: Rc<RefCell<InstrQueue>>) -> Frontend {
        Frontend {
            instr_queue,
            ip_next_fetch: -1,
            n_wide: cpu_config.frontend_n_wide,
            program_option: None,
        }
    }

    pub(crate) fn init(&mut self, program: Program) {
        self.program_option = Some(program);
        self.ip_next_fetch = 0;
    }

    pub(crate) fn do_cycle(&mut self) {
        match &self.program_option {
            None => return,
            Some(program) => {
                for _ in 0..self.n_wide {
                    if self.instr_queue.borrow_mut().is_full() {
                        break;
                    }

                    if program.code.len() == self.ip_next_fetch as usize{
                        // at the end of the program
                        return;
                    }

                    if self.ip_next_fetch == -1 {
                        break;
                    }

                    let instr = program.get(self.ip_next_fetch as usize);
                    println!("Frontend: decoded {}", instr);
                    self.instr_queue.borrow_mut().enqueue(instr);
                    self.ip_next_fetch += 1;
                }
            }
        }
    }
}
