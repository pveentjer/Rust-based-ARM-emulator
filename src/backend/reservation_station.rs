use std::fmt;
use std::fmt::Display;
use Operand::Unused;

use crate::instructions::instructions::{MAX_SINK_COUNT, MAX_SOURCE_COUNT, mnemonic, Opcode, Operand};

#[derive(Clone, Copy, PartialEq)]
pub enum RSState {
    FREE,
    BUSY,
}

pub struct RS {
    pub(crate) sb_pos: u16,
    pub(crate) rob_slot_index: u16,
    pub(crate) opcode: Opcode,
    pub(crate) state: RSState,
    pub(crate) source_cnt: u8,
    pub(crate) source: [Operand; MAX_SOURCE_COUNT as usize],
    pub(crate) source_ready_cnt: u8,
    pub(crate) sink_cnt: u8,
    pub(crate) sink: [Operand; MAX_SINK_COUNT as usize],
}

impl RS {
    fn new() -> Self {
        Self {
            opcode: Opcode::NOP,
            state: RSState::FREE,
            source_cnt: 0,
            source: [Unused, Unused, Unused],
            source_ready_cnt: 0,
            sink_cnt: 0,
            sink: [Unused, Unused],
            sb_pos: 0,
            rob_slot_index: 0,
        }
    }
}

impl Display for RS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RS ")?;
        write!(f, "{}", mnemonic(self.opcode))?;

        for k in 0..self.source_cnt {
            write!(f, " {:?}", self.source[k as usize])?;
        }

        for k in 0..self.sink_cnt {
            write!(f, " {:?}", self.sink[k as usize])?;
        }

        Ok(())
    }
}

pub(crate) struct RSTable {
    free_stack: Vec<u16>,
    ready_queue_head: u64,
    ready_queue_tail: u64,
    ready_queue: Vec<u16>,
    pub(crate) capacity: u16,
    array: Vec<RS>,
}

impl RSTable {
    pub(crate) fn new(capacity: u16) -> Self {
        let mut free_stack = Vec::with_capacity(capacity as usize);
        let mut array = Vec::with_capacity(capacity as usize);
        for i in 0..capacity {
            array.push(RS::new());
            free_stack.push(i);
        }
        let mut ready_queue = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            ready_queue.push(0);
        }

        RSTable {
            capacity,
            array,
            free_stack,
            ready_queue,
            ready_queue_head: 0,
            ready_queue_tail: 0,
        }
    }

    pub(crate) fn get_mut(&mut self, rs_index: u16) -> &mut RS {
        return &mut self.array[rs_index as usize];
    }

    pub(crate) fn enqueue_ready(&mut self, rs_index: u16) {
        let index = (self.ready_queue_tail % self.capacity as u64) as usize;
        self.ready_queue[index] = rs_index;
        self.ready_queue_tail += 1;
    }

    // todo: has_ready/dequeue_ready can be simplified by using an Option
    pub(crate) fn has_ready(&self) -> bool {
        return self.ready_queue_head != self.ready_queue_tail;
    }

    pub(crate) fn deque_ready(&mut self) -> u16 {
        assert!(self.has_ready(), "RSTable: can't dequeue ready when there are no ready items");
        let index = (self.ready_queue_head % self.capacity as u64) as u16;
        let rs_ready_index = self.ready_queue[index as usize];

        self.ready_queue_head += 1;
        return rs_ready_index;
    }

    pub(crate) fn has_free(&self) -> bool {
        return !self.free_stack.is_empty();
    }

    pub(crate) fn allocate(&mut self) -> u16 {
        if let Some(last_element) = self.free_stack.pop() {
            return last_element;
        } else {
            panic!("No free RS")
        }
    }

    pub(crate) fn deallocate(&mut self, rs_index: u16) {
        self.free_stack.push(rs_index);
    }
}

