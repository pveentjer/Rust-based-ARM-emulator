use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::fmt::Display;
use std::rc::Rc;
use Operand::Unused;

use crate::instructions::instructions::{Instr, MAX_SINK_COUNT, MAX_SOURCE_COUNT, mnemonic, Opcode, Operand};
use crate::instructions::instructions::Opcode::NOP;

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
    pub(crate) source: [Operand; MAX_SOURCE_COUNT as usize],
    pub(crate) source_ready_cnt: u8,
    pub(crate) sink_cnt: u8,
    pub(crate) sink: [Operand; MAX_SINK_COUNT as usize],
    pub(crate) index: u16,

}

impl RS {
    fn new(index: u16) -> Self {
        Self {
            opcode: Opcode::NOP,
            state: RSState::IDLE,
            source_cnt: 0,
            source: [Unused, Unused, Unused],
            source_ready_cnt: 0,
            sink_cnt: 0,
            sink: [Unused, Unused],
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
            self.sink[k as usize] = Unused;
        }

        // not needed
        for k in 0..MAX_SOURCE_COUNT {
            self.source[k as usize] = Unused;
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
        // let mut ready_queue = Vec::with_capacity(capacity as usize);
        // for _ in 0..capacity {
        //     ready_queue.push(0);
        // }

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

    // fn to_index(&self, seq: u64) -> u16 {
    //     (seq % self.capacity as u64) as u16
    // }
    //
    // fn on_ready_queue(&self, rs_index:u16)->bool{
    //     for k in self.ready_queue_head .. self.ready_queue_tail{
    //         let index =self.to_index(k);
    //         if *self.ready_queue.get(index as usize).unwrap() == rs_index{
    //             return true;
    //         }
    //     }
    //
    //     false
    // }

    pub(crate) fn get_mut(&mut self, rs_index: u16) -> &mut RS {
        // if rs_index == 6493{
        //     println!("get_mut {} ",rs_index)
        // }

        return &mut self.array[rs_index as usize];
    }

    pub(crate) fn enqueue_ready(&mut self, rs_index: u16) {
        debug_assert!(!self.ready_queue.contains(&rs_index), "Can't enqueue ready rs_index={}, it is already on the ready queue", rs_index);
        debug_assert!(self.allocated.contains(&rs_index), "Can't enqueue ready rs_index={}, it isn't in the allocated set", rs_index);

       // println!("RS enqueue_ready {}", rs_index);

        self.ready_queue.push_front(rs_index);
        //
        // let index = self.to_index(self.ready_queue_tail);
        //
        // #[cfg(debug_assertions)]
        // {
        //     let rs = &self.array[rs_index as usize];
        //     debug_assert!(rs.state == RSState::BUSY);
        //     debug_assert!(rs.rob_slot_index.is_some());
        //     debug_assert!(rs.source_cnt==rs.source_cnt);
        // }
        //
        // self.ready_queue[index as usize] = rs_index;
        // self.ready_queue_tail += 1;
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
        //
        // while self.has_ready() {
        //     let rs_index = self.deque_ready();
        //     println!("RS Flush: {}", rs_index);
        //     self.deallocate(rs_index);
        //     cnt += 1;
        // }

        for k in 0..self.capacity {
            self.array.get_mut(k as usize).unwrap().reset();
            self.idle_stack.push(k);
        }

        //println!("RS Station flush item cnt {}", cnt);
    }

    pub(crate) fn deque_ready(&mut self) -> u16 {
        debug_assert!(self.has_ready(), "RSTable: can't dequeue ready when there are no ready items");
        //let index = self.to_index(self.ready_queue_head);
        let rs_ready_index = self.ready_queue.pop_front().unwrap();

        debug_assert!(self.allocated.contains(&rs_ready_index),
                      " deque_ready for rs_ready_index {} failed, it is not in the allocated set", rs_ready_index);

        //println!("RS deque_ready {} ", rs_ready_index);

        #[cfg(debug_assertions)]
        {
            let rs = &self.array[rs_ready_index as usize];
            //println!("RS dequeue ready {:?}", rs.opcode);

            debug_assert!(rs.state == RSState::BUSY, "RS should be busy state, rs_index {}", rs_ready_index);
            debug_assert!(rs.rob_slot_index.is_some());
        }

        //self.ready_queue_head += 1;
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

            //debug_assert!(!self.on_ready_queue(rs_index));

            self.allocated.insert(rs_index);

            let mut rs = &mut self.array[rs_index as usize];

            debug_assert!(rs.state == RSState::IDLE);
            rs.state = RSState::BUSY;
            //println!("---------------------RSTable allocate {}", rs_index);
            return rs_index;
        } else {
            panic!("No free RS")
        }
    }

    pub(crate) fn deallocate(&mut self, rs_index: u16) {
        let mut rs = &mut self.array[rs_index as usize];

        // debug_assert!(!self.on_ready_queue(rs_index));

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

        //println!("RSTable deallocate {}", rs_index);

        // todo: enable
        // self.free_stack.push(rs_index);
    }
}

