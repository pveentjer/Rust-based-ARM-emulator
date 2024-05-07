use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Display;
use crate::cpu::CPUConfig;
use crate::instructions::{Instr, InstrQueue};

struct RS{

}

struct RS_Table{

}

pub(crate) struct Backend<'a> {
    instr_queue: Rc<RefCell<InstrQueue<'a>>>,
}

impl<'a> Backend<'a> {
    pub(crate) fn new(cpu_config: &'a CPUConfig, instr_queue: Rc<RefCell<InstrQueue<'a>>>) -> Backend<'a> {
        Backend { instr_queue }
    }

    pub(crate) fn cycle(&mut self) {
        loop {
            match self.instr_queue.borrow_mut().dequeue() {
                None => { return; }
                Some(instr) => {println!("{}",instr)}
            }
        }
    }
}
