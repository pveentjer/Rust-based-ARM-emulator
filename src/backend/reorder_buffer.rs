use std::rc::Rc;

use crate::instructions::instructions::{Instr, MAX_SINK_COUNT, RegisterType};

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum ROBSlotState {
    // the initial state
    IDLE,
    // the instruction is issued into the rob
    ISSUED,
    // the instruction is dispatched to an EU where it will be processed
    DISPATCHED,
    // the instruction has executed
    EXECUTED,
}

pub(crate) struct ROBSlot {
    // the pc of the current instr.
    pub(crate) pc: usize,
    pub(crate) instr: Option<Rc<Instr>>,
    pub(crate) state: ROBSlotState,
    pub(crate) index: u16,
    pub(crate) rs_index: Option<u16>,
    pub(crate) sink_phys_regs: [Option<RegisterType>; MAX_SINK_COUNT as usize],
    pub(crate) branch_target_predicted: usize,
    pub(crate) branch_target_actual: usize,
    pub(crate) sb_pos: Option<u16>,
    pub(crate) eu_index: Option<u8>,
}

impl ROBSlot {
    fn reset(&mut self) {
        self.branch_target_predicted = 0;
        self.branch_target_actual = 0;
        self.state = ROBSlotState::IDLE;
        self.rs_index = None;
        self.instr = None;
        self.sb_pos = None;
        self.eu_index = None;
        self.pc = 0;

        for k in 0..MAX_SINK_COUNT {
            self.sink_phys_regs[k as usize] = None;
        }
    }
}

pub(crate) struct ROB {
    pub(crate) capacity: u16,
    pub(crate) seq_issued: u64,
    pub(crate) seq_dispatched: u64,
    pub(crate) seq_rs_allocated: u64,
    pub(crate) seq_retired: u64,
    pub(crate) head: u64,
    pub(crate) tail: u64,
    pub(crate) slots: Vec<ROBSlot>,
}

impl ROB {
    pub(crate) fn new(capacity: u16) -> Self {
        let mut slots = Vec::with_capacity(capacity as usize);
        for k in 0..capacity {
            slots.push(ROBSlot {
                index: k,
                instr: None,
                state: ROBSlotState::IDLE,
                rs_index: None,
                sink_phys_regs: [None, None],
                branch_target_predicted: 0,
                branch_target_actual: 0,
                sb_pos: None,
                eu_index: None,
                pc: 0,
            });
        }

        Self {
            capacity,
            seq_issued: 0,
            seq_dispatched: 0,
            seq_rs_allocated: 0,
            seq_retired: 0,
            tail: 0,
            head: 0,
            slots,
        }
    }

    pub(crate) fn get_mut(&mut self, slot_index: u16) -> &mut ROBSlot {
        // todo: should be between head and tail
        &mut self.slots[slot_index as usize]
    }

    pub(crate) fn allocate(&mut self) -> u16 {
        debug_assert!(self.has_space(), "ROB: Can't allocate if the ROB has no space.");

        let index = self.to_index(self.tail);

        self.tail += 1;
        return index;
    }

    pub(crate) fn to_index(&self, seq: u64) -> u16 {
        (seq % self.capacity as u64) as u16
    }

    pub(crate) fn deallocate(&mut self) {
        debug_assert!(!self.is_empty(), "ROB: Can't deallocate if ROB is empty");

        let index = self.to_index(self.head) as usize;
        self.slots[index].reset();
        self.head += 1;
    }

    pub(crate) fn size(&self) -> u16 {
        return (self.tail - self.head) as u16;
    }

    pub(crate) fn is_empty(&self) -> bool {
        return self.head == self.tail;
    }

    pub(crate) fn has_space(&self) -> bool {
        return self.capacity > self.size();
    }

    pub(crate) fn flush(&mut self) {
        // todo: we don't need to go over the whole rob; just over the busy slots
        for i in 0..self.capacity {
            let slot = &mut self.slots[i as usize];
            slot.reset();
        }
        self.head = 0;
        self.tail = 0;
        self.seq_retired = 0;
        self.seq_issued = 0;
        self.seq_rs_allocated = 0;
        self.seq_dispatched = 0;
    }
}
