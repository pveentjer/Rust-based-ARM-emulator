use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Display;
use crate::cpu::CPUConfig;
use crate::instructions::{Instr, InstrQueue, Opcode, Operand, OpType, OpUnion};
use crate::instructions::Opcode::NOP;

enum RS_State{
    UNUSED,
}

struct RS {
    pub(crate) opcode: Opcode,
    pub(crate) state: RS_State,
    pub(crate) sink_cnt: u8,
    pub(crate) sink: [Operand; crate::instructions::MAX_SINK_COUNT as usize],
    pub(crate) source_cnt: u8,
    pub(crate) source: [Operand; crate::instructions::MAX_SOURCE_COUNT as usize],
}

impl RS {
    pub fn new() -> Self {
        Self {
            opcode: Opcode::NOP,
            state: RS_State::UNUSED,
            source_cnt: 0,
            source: [
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused },
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
            ],
            sink_cnt: 0,
            sink: [
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
            ],
        }
    }
}

struct RS_Table {
    count: u16,
    array: Vec<RS>,
}

impl RS_Table {
    pub(crate) fn new(rs_count: u16) -> RS_Table {
        let mut array = Vec::with_capacity(rs_count as usize);
        for i in 0..rs_count{
            array.push(RS::new());
        }
        RS_Table {
            count: rs_count,
            array,
        }
    }
}

pub(crate) struct Backend<'a> {
    instr_queue: Rc<RefCell<InstrQueue<'a>>>,
    rs_table: RS_Table,
}

impl<'a> Backend<'a> {
    pub(crate) fn new(cpu_config: &'a CPUConfig, instr_queue: Rc<RefCell<InstrQueue<'a>>>) -> Backend<'a> {
        Backend {
            instr_queue,
            rs_table: RS_Table::new(cpu_config.rs_count)
        }
    }

    pub(crate) fn cycle(&mut self) {
        loop {
            match self.instr_queue.borrow_mut().dequeue() {
                None => { return; }
                Some(instr) => { println!("{}", instr) }
            }
        }
    }
}
