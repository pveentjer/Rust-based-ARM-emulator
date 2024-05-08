use std::rc::Rc;
use std::cell::RefCell;
use crate::cpu::CPUConfig;
use crate::instructions::{Instr, InstrQueue, Program};

pub(crate) struct Frontend<'a> {
    instr_queue: Rc<RefCell<InstrQueue<'a>>>,
    n_wide: u8,
    ip_next_fetch: i64,
    program_option: Option<Program>,
}

impl<'a> Frontend<'a> {
    pub(crate) fn new(cpu_config: & CPUConfig, instr_queue: Rc<RefCell<InstrQueue<'a>>>) -> Frontend<'a> {
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

                    // ugly raw pointers, but at least we are unstuck for now
                    let instr_ptr = &program.code[self.ip_next_fetch as usize] as *const Instr;
                    self.instr_queue.borrow_mut().enqueue(unsafe { &*instr_ptr });
                    self.ip_next_fetch += 1;
                }
            }
        }
    }
}
