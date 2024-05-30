use std::rc::Rc;
use Operand::Unused;

use crate::instructions::instructions::{Instr, MAX_SINK_COUNT, Operand, WordType};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ROBSlotState {
    // the initial state
    IDLE,
    // the instruction is issued into the rob
    ISSUED,
    // the instruction has been issued and has a reservation station
    ISSUED_WITH_RS,
    // the instruction is dispatched to an EU where it will be processed
    DISPATCHED,
    // rhw instruction has executed
    EXECUTED,
}

pub(crate) struct ROBSlot {
    pub(crate) instr: Option<Rc<Instr>>,
    pub(crate) state: ROBSlotState,
    pub(crate) index: u16,
    pub(crate) result: Vec<WordType>,
    pub(crate) rs_index: u16,
    pub(crate) sink: [Operand; MAX_SINK_COUNT as usize],
    pub(crate) invalidated: bool,
    pub(crate) branch_target_predicted: usize,
    pub(crate) branch_target_actual: usize,
    pub(crate) sb_pos: u16,
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
    pub(crate) fn new(capacity: u16) -> Self {
        let mut slots = Vec::with_capacity(capacity as usize);
        for k in 0..capacity {
            slots.push(ROBSlot {
                index: k,
                instr: None,
                state: ROBSlotState::IDLE,
                result: Vec::with_capacity(MAX_SINK_COUNT as usize),
                rs_index: 0,
                sink: [Unused, Unused],
                invalidated: false,
                branch_target_predicted: 0,
                branch_target_actual: 0,
                sb_pos: 0,
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

    pub(crate) fn allocate(&mut self) -> u16 {
        assert!(self.has_space(), "ROB: Can't allocate if no space.");

        let index = (self.tail % self.capacity as u64) as u16;
        self.tail += 1;
        return index;
    }

    // Are there any rob entries that have been issued, but have not yet been dispatched.
    pub(crate) fn has_issued(&self) -> bool {
        return self.tail > self.issued;
    }

    pub(crate) fn next_issued(&mut self) -> u16 {
        assert!(self.has_issued(), "ROB: can't issue next since there are none");
        let index = (self.issued % self.capacity as u64) as u16;
        self.issued += 1;
        return index;
    }

    pub(crate) fn head_has_executed(&self) -> bool {
        // todo: we should not passed issued
        // we should not pass the head
        if self.tail == self.head {
            return false;
        }

        let index = (self.head % self.capacity as u64) as u16;
        let rob_slot = &self.slots[index as usize];
        return rob_slot.state == ROBSlotState::EXECUTED;
    }

    pub(crate) fn last_executed(&self)->u16{
        if self.tail == self.head {
            panic!();
        }

        (self.head % self.capacity as u64) as u16
    }

    pub(crate) fn next_executed(&mut self) -> u16 {
        assert!(self.head_has_executed(), "ROB: can't next_executed because there are no slots in executed state");

        let index = (self.head % self.capacity as u64) as u16;
        self.head += 1;
        return index;
    }

    pub(crate) fn size(&self) -> u16 {
        return (self.tail - self.head) as u16;
    }

    pub(crate) fn has_space(&self) -> bool {
        return self.capacity > self.size();
    }
}
