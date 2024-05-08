use std::rc::Rc;
use std::cell::RefCell;
use std::cmp::PartialEq;
use std::fmt;
use std::fmt::Display;
use crate::cpu::{ArgRegFile, CPUConfig};
use crate::instructions::{Instr, InstrQueue, mnemonic, Opcode, Operand, OpType, OpUnion, RegisterType, WordType};
use crate::memory_subsystem::MemorySubsystem;

struct PhysReg {
    value: WordType,
    has_value: bool,
    index: u16,
}

struct PhysRegFile {
    free_stack: Vec<u16>,
    count: u16,
    registers: Vec<PhysReg>,
}

impl PhysRegFile {
    fn new(phys_reg_count: u16) -> PhysRegFile {
        let mut free_stack = Vec::with_capacity(phys_reg_count as usize);
        let mut array = Vec::with_capacity(phys_reg_count as usize);
        for i in 0..phys_reg_count {
            array.push(PhysReg { value: 0, has_value: false, index: i });
            free_stack.push(i);
        }

        PhysRegFile {
            count: phys_reg_count,
            registers: array,
            free_stack,
        }
    }

    fn has_free(&self) -> bool {
        return !self.free_stack.is_empty();
    }

    fn get(&self, reg: RegisterType) -> &PhysReg {
        return self.registers.get(reg as usize).unwrap();
    }

    fn get_mut(&mut self, reg: RegisterType) -> &mut PhysReg {
        return self.registers.get_mut(reg as usize).unwrap();
    }

    fn allocate(&mut self) -> RegisterType {
        if let Some(last_element) = self.free_stack.pop() {
            return last_element;
        } else {
            panic!("No free PhysReg")
        }
    }

    fn deallocate(&mut self, reg: RegisterType) {
        self.free_stack.push(reg);
    }

    //pub
}

enum RSState {
    FREE,
    BUSY,
}

struct RS {
    index: u16,
    opcode: Opcode,
    state: RSState,
    sink_available: bool,
    sink: Operand,
    source_cnt: u8,
    source: [Operand; crate::instructions::MAX_SOURCE_COUNT as usize],
    source_ready_cnt: u8,
    sb_pos: u16,
}

impl RS {
    fn new(index: u16) -> Self {
        Self {
            index,
            opcode: Opcode::NOP,
            state: RSState::FREE,
            source_cnt: 0,
            source: [
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused },
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
            ],
            source_ready_cnt: 0,
            sink_available: false,
            sink: Operand { op_type: OpType::UNUSED, union: OpUnion::Unused },
            sb_pos: 0,
        }
    }
}

impl Display for RS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RS ")?;
        write!(f, "{}", mnemonic(&self.opcode))?;

        for k in 0..self.source_cnt {
            write!(f, " {}", self.source[k as usize])?;
        }

        if self.sink_available {
            write!(f, " {}", self.sink)?;
        }

        Ok(())
    }
}

struct RSTable {
    free_stack: Vec<u16>,
    ready_queue_head: u64,
    ready_queue_tail: u64,
    ready_queue: Vec<u16>,
    count: u16,
    array: Vec<RS>,
}

impl RSTable {
    fn new(rs_count: u16) -> Self {
        let mut free_stack = Vec::with_capacity(rs_count as usize);
        let mut array = Vec::with_capacity(rs_count as usize);
        for i in 0..rs_count {
            array.push(RS::new(i));
            free_stack.push(i);
        }
        let mut ready_queue = Vec::with_capacity(rs_count as usize);
        for _ in 0..rs_count {
            ready_queue.push(0);
        }

        RSTable {
            count: rs_count,
            array,
            free_stack,
            ready_queue,
            ready_queue_head: 0,
            ready_queue_tail: 0,
        }
    }

    fn get_mut(&mut self, sb_index: u16) -> &mut RS {
        return &mut self.array[sb_index as usize];
    }

    fn enqueue_ready(&mut self, sb_index: u16) {
        let index = (self.ready_queue_tail % self.count as u64) as usize;
        self.ready_queue[index] = sb_index;
        self.ready_queue_tail += 1;
    }

    // todo: has_ready/dequeue_ready can be simplified by using an Option
    fn has_ready(&self) -> bool {
        return self.ready_queue_head != self.ready_queue_tail;
    }

    fn deque_ready(&mut self) -> u16 {
        assert!(self.has_ready(), "RSTable: can't dequeue ready when there are no ready items");
        let index = (self.ready_queue_head % self.count as u64) as u16;
        self.ready_queue_head += 1;
        return index;
    }

    fn has_free(&self) -> bool {
        return !self.free_stack.is_empty();
    }

    fn allocate(&mut self) -> u16 {
        if let Some(last_element) = self.free_stack.pop() {
            return last_element;
        } else {
            panic!("No free RS")
        }
    }

    fn deallocate(&mut self, rs_index: u16) {
        self.free_stack.push(rs_index);
    }

    //pub
}

struct RATEntry {
    phys_reg: RegisterType,
    // True of this entry is currently in use.
    valid: bool,
}

struct RAT {
    pub(crate) table: Vec<RATEntry>,
}

impl RAT {
    pub fn new(phys_reg_count: u16) -> Self {
        let mut table = Vec::with_capacity(phys_reg_count as usize);
        for _ in 0..phys_reg_count {
            table.push(RATEntry { phys_reg: 0, valid: false });
        }
        Self { table }
    }

    pub fn get(&self, arch_reg: RegisterType) -> &RATEntry {
        return self.table.get(arch_reg as usize).unwrap();
    }

    pub fn get_mut(&mut self, arch_reg: RegisterType) -> &mut RATEntry {
        return self.table.get_mut(arch_reg as usize).unwrap();
    }
}


enum ROBSlotState {
    UNUSED,
    ISSUED,
    DISPATCHED,
    EXECUTED,
}

struct ROBSlot {
    instr: Option<Rc<Instr>>,
    state: ROBSlotState,
    index: u16,
    rb_slot_index: Option<u16>,
}

struct ROB {
    capacity: u16,
    issued: u64,
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
            });
        }

        Self {
            capacity,
            head: 0,
            issued: 0,
            tail: 0,
            slots,
        }
    }

    fn get_mut(&mut self, slot_index: u16) -> &mut ROBSlot {
        &mut self.slots[slot_index as usize]
    }

    fn allocate(&mut self) -> u16 {
        assert!(self.has_space(), "ROB: Can't allocate if no space.");

        let index = (self.tail % self.capacity as u64) as u16;
        self.tail += 1;
        return index;
    }

    // Are there any rob entries that have been issued, but have not yet been dispatched.
    fn has_issued(&self) -> bool {
        return self.tail > self.issued;
    }

    fn next_issued(&mut self) -> u16 {
        assert!(self.has_issued(), "ROB: can't issue next since there are none");

        println!("next issued with success");

        let index = (self.issued % self.capacity as u64) as u16;
        self.issued += 1;
        return index;
    }

    fn size(&self) -> u16 {
        return (self.tail - self.head) as u16;
    }

    fn has_space(&self) -> bool {
        return self.capacity > self.size();
    }

}


pub(crate) struct Backend {
    instr_queue: Rc<RefCell<InstrQueue>>,
    rs_table: Rc<RefCell<RSTable>>,
    phys_reg_file: Rc<RefCell<PhysRegFile>>,
    arch_reg_file: Rc<RefCell<ArgRegFile>>,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    rat: Rc<RefCell<RAT>>,
    rob: Rc<RefCell<ROB>>,
}

impl PartialEq for OpType {
    fn eq(&self, other: &Self) -> bool {
        return self.eq(other);
    }
}

impl Backend {
    pub(crate) fn new(cpu_config: &CPUConfig,
                      instr_queue: Rc<RefCell<InstrQueue>>,
                      memory_subsystem: Rc<RefCell<MemorySubsystem>>,
                      arch_reg_file: Rc<RefCell<ArgRegFile>>) -> Backend {
        Backend {
            instr_queue,
            memory_subsystem,
            arch_reg_file,
            rs_table: Rc::new(RefCell::new(RSTable::new(cpu_config.rs_count))),
            phys_reg_file: Rc::new(RefCell::new(PhysRegFile::new(cpu_config.phys_reg_count))),
            rat: Rc::new(RefCell::new(RAT::new(cpu_config.phys_reg_count))),
            rob: Rc::new(RefCell::new(ROB::new(cpu_config.rob_capacity))),
        }
    }

    pub(crate) fn do_cycle(&mut self) {
        //  self.cycle_retire();
        //self.cycle_dispatch();
        self.cycle_issue();
    }

    fn cycle_retire(&mut self) {}

    fn cycle_dispatch(&mut self) {
        let mut rs_table = self.rs_table.borrow_mut();
        let mut rob = self.rob.borrow_mut();
        while rob.has_issued() && rs_table.has_free() {
            let rs_index = rs_table.deque_ready();
            let rs = rs_table.get_mut(rs_index);

            let instr = self.instr_queue.borrow().peek();

            let mut rs_table = self.rs_table.borrow_mut();


            println!("Dispatched {}", instr);

            // if rs.source_ready_cnt == rs.source_cnt {
            //     rs_table.enqueue_ready(rs.index);
            // }

            self.instr_queue.borrow_mut().dequeue();
            // Process the dequeued RS
            // println!("dispatch {}", rs);
            //  let junk = &mut RS::new(1);
            //  self.rs_table.borrow_mut().deallocate(junk);
        }
    }

    fn cycle_issue(&mut self) {
        let mut rob = self.rob.borrow_mut();
        let mut rs_table = self.rs_table.borrow_mut();
        let mut instr_queue = self.instr_queue.borrow_mut();
        let mut phys_reg_file = self.phys_reg_file.borrow_mut();
        let mut rat = self.rat.borrow_mut();
        let arch_reg_file = self.arch_reg_file.borrow();

        // try to put as many instructions into the rob as possible.
        while !instr_queue.is_empty()  && rob.has_space() {
            let instr = instr_queue.peek();

            let rb_slot = -1;

            // if instr.sink_available && instr.sink.op_type == OpType::MEMORY {
            //     if !self.memory_subsystem.borrow().sb.has_space() {
            //         return;
            //     }
            // }

            instr_queue.dequeue();

            let rob_slot_index = rob.allocate();
            let rob_slot = rob.get_mut(rob_slot_index);

            println!("Issued {}", instr);

            rob_slot.state = ROBSlotState::DISPATCHED;
            rob_slot.instr = Some(instr);
        }

        print!("rob.has_issued {}\n", rob.has_issued());
        print!("rs_table.has_free {}\n", rs_table.has_free());

        // try to put as many instructions from the rob, into reservation stations as possible.
        while rob.has_issued() && rs_table.has_free() {
            println!("=====================================================================");

            let rob_slot_index = rob.next_issued();
            let mut rob_slot = rob.get_mut(rob_slot_index);
            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);
            let rs_index = rs_table.allocate();
            let mut rs = rs_table.get_mut(rs_index);
            println!("Dispatched {}", instr);

            rs.state = RSState::BUSY;
            rs.sink_available = instr.sink_available;
            if instr.sink_available {
                let op_instr = &instr.sink;
                let op_rs = &mut rs.sink;

                match op_instr.op_type {
                    OpType::REGISTER => {
                        let arch_reg = op_instr.union.get_register();
                        let phys_reg = phys_reg_file.allocate();
                        let rat_entry = rat.get_mut(arch_reg);
                        rat_entry.phys_reg = phys_reg;
                        rat_entry.valid = true;
                        op_rs.op_type = OpType::REGISTER;
                        op_rs.union = OpUnion::Register(rat_entry.phys_reg);
                    }
                    OpType::MEMORY => {
                        rs.sink = *op_instr;
                        // todo: not handling a full sb.
                        // since the instructions are issued in program order, a slot is allocated in the
                        // sb in program order. And since sb will commit to the coherent cache
                        // (in this case directly to memory), the stores will become visible
                        // in program order.
                        rs.sb_pos = self.memory_subsystem.borrow_mut().sb.allocate();
                    }
                    OpType::VALUE => {
                        panic!("Can't have a value as sink {}", op_rs)
                    }
                    OpType::UNUSED =>
                        panic!("Unrecognized {}", op_rs)
                }
            }

            rs.source_cnt = instr.source_cnt;
            rs.source_ready_cnt = 0;
            for i in 0..instr.source_cnt {
                let instr_op = &instr.source[i as usize];
                let rs_op = &mut rs.source[i as usize];
                match instr_op.op_type {
                    OpType::REGISTER => {
                        let arch_reg = instr_op.union.get_register();
                        let rat_entry = rat.get(arch_reg);
                        if rat_entry.valid {
                            let phys_reg_struct = phys_reg_file.get(rat_entry.phys_reg);
                            if phys_reg_struct.has_value {
                                let value = phys_reg_struct.value;
                                rs_op.op_type = OpType::VALUE;
                                rs_op.union = OpUnion::Constant(value);
                                rs.source_ready_cnt += 1;
                            } else {
                                rs_op.op_type = OpType::REGISTER;
                                rs_op.union = OpUnion::Register(rat_entry.phys_reg);
                            }
                        } else {
                            let value = arch_reg_file.get_value(arch_reg);
                            rs_op.op_type = OpType::VALUE;
                            rs_op.union = OpUnion::Constant(value);
                            rs.source_ready_cnt += 1;
                        }
                    }
                    OpType::MEMORY => {
                        rs.source[i as usize] = *instr_op;
                        rs.source_ready_cnt += 1;
                    }
                    OpType::VALUE => {
                        rs.source[i as usize] = *instr_op;
                        rs.source_ready_cnt += 1;
                    }
                    OpType::UNUSED =>
                        panic!("Unrecognized {}", rs_op)
                }
            }

            if rs.source_ready_cnt == rs.source_cnt {
                rs_table.enqueue_ready(rs_index);
            }
        }
    }
}
