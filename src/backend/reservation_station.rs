use std::collections::{HashSet, VecDeque};

use crate::instructions::instructions::{DWordType, MAX_SINK_COUNT, MAX_SOURCE_COUNT, Opcode, Operand, RegisterType};
use crate::instructions::instructions::Opcode::NOP;

pub(crate) struct RSOperand {
    pub(crate) operand: Option<Operand>,
    pub(crate) value: Option<DWordType>,
    pub(crate) phys_reg: Option<RegisterType>,
}

impl RSOperand {
    fn new() -> RSOperand {
        RSOperand { operand: None, value: None, phys_reg: None }
    }

    fn reset(&mut self) {
        self.operand = None;
        self.value = None;
        self.phys_reg = None;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum RSState {
    IDLE,
    BUSY,
}


// A single reservation station
pub(crate) struct RS {
    pub(crate) rob_slot_index: Option<u16>,
    pub(crate) opcode: Opcode,
    pub(crate) state: RSState,
    pub(crate) source_cnt: u8,
    pub(crate) source: [RSOperand; MAX_SOURCE_COUNT as usize],
    pub(crate) source_ready_cnt: u8,
    pub(crate) sink_cnt: u8,
    pub(crate) sink: [RSOperand; MAX_SINK_COUNT as usize],
    pub(crate) index: u16,

}

impl RS {
    fn new(index: u16) -> Self {
        Self {
            opcode: Opcode::NOP,
            state: RSState::IDLE,
            source_cnt: 0,
            source: [RSOperand::new(), RSOperand::new(), RSOperand::new()],
            source_ready_cnt: 0,
            sink_cnt: 0,
            sink: [RSOperand::new(), RSOperand::new()],
            rob_slot_index: None,
            index,
        }
    }

    fn reset(&mut self) {
        self.rob_slot_index = None;
        self.opcode = NOP;
        self.state = RSState::IDLE;
        self.source_cnt = 0;
        self.source_ready_cnt = 0;
        self.sink_cnt = 0;

        // not needed
        for k in 0..MAX_SINK_COUNT {
            self.sink[k as usize].reset();
        }

        // not needed
        for k in 0..MAX_SOURCE_COUNT {
            self.source[k as usize].reset();
        }
    }
}

pub(crate) struct RSTable {
    idle_stack: Vec<u16>,
    // ready_queue_head: u64,
    // ready_queue_tail: u64,
    // ready_queue: Vec<u16>,
    ready_queue: VecDeque<u16>,
    pub(crate) capacity: u16,
    array: Vec<RS>,
    // delete
    pub(crate) allocated: HashSet<u16>,
}

impl RSTable {
    pub(crate) fn new(capacity: u16) -> Self {
        let mut free_stack = Vec::with_capacity(capacity as usize);
        let mut array = Vec::with_capacity(capacity as usize);
        for i in 0..capacity {
            array.push(RS::new(i));
            free_stack.push(i);
        }


        RSTable {
            capacity,
            array,
            idle_stack: free_stack,
            ready_queue: VecDeque::new(),
            //ready_queue_head: 0,
            //ready_queue_tail: 0,
            allocated: HashSet::new(),
        }
    }

    pub(crate) fn get_mut(&mut self, rs_index: u16) -> &mut RS {
        return &mut self.array[rs_index as usize];
    }

    pub(crate) fn enqueue_ready(&mut self, rs_index: u16) {
        debug_assert!(!self.ready_queue.contains(&rs_index), "Can't enqueue ready rs_index={}, it is already on the ready queue", rs_index);
        debug_assert!(self.allocated.contains(&rs_index), "Can't enqueue ready rs_index={}, it isn't in the allocated set", rs_index);

        self.ready_queue.push_front(rs_index);
    }

    // todo: has_ready/dequeue_ready can be simplified by using an Option
    pub(crate) fn has_ready(&self) -> bool {
        !self.ready_queue.is_empty()

        //return self.ready_queue_head != self.ready_queue_tail;
    }

    pub(crate) fn flush(&mut self) {
        self.ready_queue.clear();
        self.idle_stack.clear();
        self.allocated.clear();

        for k in 0..self.capacity {
            self.array.get_mut(k as usize).unwrap().reset();
            self.idle_stack.push(k);
        }
    }

    pub(crate) fn deque_ready(&mut self) -> u16 {
        debug_assert!(self.has_ready(), "RSTable: can't dequeue ready when there are no ready items");
        //let index = self.to_index(self.ready_queue_head);
        let rs_ready_index = self.ready_queue.pop_front().unwrap();

        debug_assert!(self.allocated.contains(&rs_ready_index),
                      " deque_ready for rs_ready_index {} failed, it is not in the allocated set", rs_ready_index);

        #[cfg(debug_assertions)]
        {
            let rs = &self.array[rs_ready_index as usize];
            //println!("RS dequeue ready {:?}", rs.opcode);

            debug_assert!(rs.state == RSState::BUSY, "RS should be busy state, rs_index {}", rs_ready_index);
            debug_assert!(rs.rob_slot_index.is_some());
        }

        return rs_ready_index;
    }

    pub(crate) fn has_idle(&self) -> bool {
        return !self.idle_stack.is_empty();
    }

    pub(crate) fn allocate(&mut self) -> u16 {
        if let Some(rs_index) = self.idle_stack.pop() {
            if self.allocated.contains(&rs_index) {
                panic!("Duplicate allocation {}", rs_index);
            }

            self.allocated.insert(rs_index);

            let rs = &mut self.array[rs_index as usize];

            debug_assert!(rs.state == RSState::IDLE);
            rs.state = RSState::BUSY;
            return rs_index;
        } else {
            panic!("No free RS")
        }
    }

    pub(crate) fn deallocate(&mut self, rs_index: u16) {
        let rs = &mut self.array[rs_index as usize];

        debug_assert!(!self.ready_queue.contains(&rs_index),
                      "rs_index {} can't be deallocated if it is still on the ready queue", rs_index);

        if !self.allocated.contains(&rs_index) {
            panic!("Deallocate while not allocated {}", rs_index);
        }

        self.allocated.remove(&rs_index);

        debug_assert!(rs_index == rs.index);


        debug_assert!(rs.state == RSState::BUSY);
        debug_assert!(!self.idle_stack.contains(&rs_index));
        rs.reset();

        self.idle_stack.push(rs_index);
    }
}

