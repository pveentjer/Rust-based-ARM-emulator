use std::collections::{HashSet, VecDeque};

use crate::instructions::instructions::{ConditionCode, DWordType, Opcode, RegisterType};
use crate::instructions::instructions::Opcode::NOP;

#[derive(Clone)]
pub(crate) struct RenamedRegister {
    pub(crate) phys_reg: Option<RegisterType>,
    pub(crate) arch_reg: RegisterType,
    pub(crate) value: Option<DWordType>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum RSState {
    IDLE,
    BUSY,
}

pub enum RSOperand2 {
    Immediate {
        value: DWordType,
    },
    Register {
        register: RenamedRegister,
    },
    Unused(),
}

impl RSOperand2 {
    pub fn value(&self) -> DWordType {
        match self {
            RSOperand2::Immediate { value } => *value,
            RSOperand2::Register { register } => register.value.unwrap(),
            RSOperand2::Unused() => panic!(),
        }
    }
}

pub struct RSDataProcessing {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub rn: Option<RenamedRegister>,
    pub rd: RenamedRegister,
    // the original value of the rd register (needed for condition codes)
    pub rd_src: Option<RenamedRegister>,
    // the cpsr for condition codes
    pub cpsr: Option<RenamedRegister>,
    pub operand2: RSOperand2,
}

pub enum RSBranchTarget {
    Immediate {
        offset: u32,
    },
    Register {
        register: RenamedRegister,
    },
}

impl RSBranchTarget {
    pub fn value(&self) -> u32 {
        match self {
            RSBranchTarget::Immediate { offset } => *offset,
            RSBranchTarget::Register { register } => register.value.unwrap() as u32,
        }
    }
}

pub struct RSBranch {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub lr: Option<RenamedRegister>,
    pub target: RSBranchTarget,
    pub rt: Option<RenamedRegister>,
}

pub struct RSLoadStore {
    pub opcode: Opcode,
    pub condition: ConditionCode,
    pub rn: RenamedRegister,
    pub rd: RenamedRegister,
    pub offset: u16,
}

pub struct RSPrintr {
    pub rn: RenamedRegister,
}

pub struct RSSynchronization {
    pub opcode: Opcode,
}

pub(crate) enum RSInstr {
    DataProcessing {
        data_processing: RSDataProcessing,
    },

    Branch {
        branch: RSBranch,
    },

    LoadStore {
        load_store: RSLoadStore,
    },

    Printr {
        printr: RSPrintr,
    },

    Synchronization {
        synchronization: RSSynchronization,
    },
}

// A single reservation station
pub(crate) struct RS {
    pub(crate) rob_slot_index: Option<u16>,
    pub(crate) opcode: Opcode,
    pub(crate) state: RSState,
    pub(crate) pending_cnt: u8,
    pub(crate) instr: RSInstr,
    pub(crate) index: u16,
}

impl RS {
    fn new(index: u16) -> Self {
        Self {
            opcode: NOP,
            state: RSState::IDLE,
            pending_cnt: 0,
            rob_slot_index: None,
            index,
            instr: RSInstr::Synchronization {
                synchronization: RSSynchronization { opcode: NOP },
            },
        }
    }

    fn reset(&mut self) {
        self.rob_slot_index = None;
        self.opcode = NOP;
        self.state = RSState::IDLE;
        self.pending_cnt = 0;
        self.instr = RSInstr::Synchronization {
            synchronization: RSSynchronization { opcode: NOP }
        };
    }
}

pub(crate) struct RSTable {
    idle_stack: Vec<u16>,
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

