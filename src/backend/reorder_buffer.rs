use std::rc::Rc;

use crate::instructions::instructions::{Instr, MAX_SINK_COUNT, Operand, WordType};

#[derive(Clone, Copy, PartialEq)]
pub enum ROBSlotState {
    UNUSED,
    ISSUED,
    DISPATCHED,
    EXECUTED,
}

pub struct ROBSlot {
    pub instr: Option<Rc<Instr>>,
    pub state: ROBSlotState,
    pub index: u16,
    //todo: not used
    pub rb_slot_index: Option<u16>,
    pub result: Vec<WordType>,
    pub rs_index: u16,
    pub sink: [Operand; MAX_SINK_COUNT as usize],
}

pub(crate) struct ROB {
    capacity: u16,
    issued: u64,
    // everything before this point is retired.
    head: u64,
    tail: u64,
    slots: Vec<ROBSlot>,
}

impl ROB {
    pub fn new(capacity: u16) -> Self {
        let mut slots = Vec::with_capacity(capacity as usize);
        for k in 0..capacity {
            slots.push(ROBSlot {
                index: k,
                instr: None,
                state: ROBSlotState::UNUSED,
                rb_slot_index: None,
                result: Vec::with_capacity(MAX_SINK_COUNT as usize),
                rs_index: 0,
                sink: [Operand::Unused, Operand::Unused],
            });
        }

        Self {
            capacity,
            issued: 0,
            tail: 0,
            head: 0,
            slots,
        }
    }

    pub(crate) fn get_mut(&mut self, slot_index: u16) -> &mut ROBSlot {
        &mut self.slots[slot_index as usize]
    }

    pub fn allocate(&mut self) -> u16 {
        assert!(self.has_space(), "ROB: Can't allocate if no space.");

        let index = (self.tail % self.capacity as u64) as u16;
        self.tail += 1;
        return index;
    }

    // Are there any rob entries that have been issued, but have not yet been dispatched.
    pub fn has_issued(&self) -> bool {
        return self.tail > self.issued;
    }

    pub fn next_issued(&mut self) -> u16 {
        assert!(self.has_issued(), "ROB: can't issue next since there are none");
        let index = (self.issued % self.capacity as u64) as u16;
        self.issued += 1;
        return index;
    }

    pub fn head_has_executed(&self) -> bool {
        // todo: we should not passed issued
        // we should not pass the head
        if self.tail == self.head {
            return false;
        }

        let index = (self.head % self.capacity as u64) as u16;
        let rob_slot = &self.slots[index as usize];
        return rob_slot.state == ROBSlotState::EXECUTED;
    }

    pub fn next_executed(&mut self) -> u16 {
        assert!(self.head_has_executed(), "ROB: can't next_retire because there are no slots retired");

        let index = (self.head % self.capacity as u64) as u16;
        self.head += 1;
        return index;
    }

    pub fn size(&self) -> u16 {
        return (self.tail - self.head) as u16;
    }

    pub fn has_space(&self) -> bool {
        return self.capacity > self.size();
    }
}
