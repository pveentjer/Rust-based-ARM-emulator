use std::rc::Rc;
use std::cell::RefCell;
use std::cmp::PartialEq;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::cpu::{ArgRegFile, CPUConfig};
use crate::instructions::{Instr, InstrQueue, mnemonic, Opcode, Operand, OpType, OpUnion, RegisterType, WordType};
use crate::memory_subsystem::MemorySubsystem;

struct PhysRegEntry {
    value: WordType,
    has_value: bool,
}

struct PhysRegFile {
    free_stack: Vec<u16>,
    count: u16,
    entries: Vec<PhysRegEntry>,
}

impl PhysRegFile {
    fn new(count: u16) -> PhysRegFile {
        let mut free_stack = Vec::with_capacity(count as usize);
        let mut entries = Vec::with_capacity(count as usize);
        for i in 0..count {
            entries.push(PhysRegEntry { value: 0, has_value: false});
            free_stack.push(i);
        }

        PhysRegFile { count, entries, free_stack}
    }

    fn has_free(&self) -> bool {
        return !self.free_stack.is_empty();
    }

    fn get(&self, reg: RegisterType) -> &PhysRegEntry {
        return self.entries.get(reg as usize).unwrap();
    }

    fn get_mut(&mut self, reg: RegisterType) -> &mut PhysRegEntry {
        return self.entries.get_mut(reg as usize).unwrap();
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

#[derive(Clone, Copy, PartialEq)]
enum RSState {
    FREE,
    BUSY,
}

struct RS {
    index: u16,
    opcode: Opcode,
    state: RSState,
    sink: Operand,
    source_cnt: u8,
    source: [Operand; crate::instructions::MAX_SOURCE_COUNT as usize],
    source_ready_cnt: u8,
    sb_pos: u16,
    rob_slot_index: u16,
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
            sink: Operand { op_type: OpType::UNUSED, union: OpUnion::Unused },
            sb_pos: 0,
            rob_slot_index: 0,
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

        if self.sink.op_type != OpType::UNUSED {
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
    capacity: u16,
    array: Vec<RS>,
}

impl RSTable {
    fn new(capacity: u16) -> Self {
        let mut free_stack = Vec::with_capacity(capacity as usize);
        let mut array = Vec::with_capacity(capacity as usize);
        for i in 0..capacity {
            array.push(RS::new(i));
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

    fn get_mut(&mut self, rs_index: u16) -> &mut RS {
        return &mut self.array[rs_index as usize];
    }

    fn enqueue_ready(&mut self, rs_index: u16) {
        let index = (self.ready_queue_tail % self.capacity as u64) as usize;
        self.ready_queue[index] = rs_index;
        self.ready_queue_tail += 1;
    }

    // todo: has_ready/dequeue_ready can be simplified by using an Option
    fn has_ready(&self) -> bool {
        return self.ready_queue_head != self.ready_queue_tail;
    }

    fn deque_ready(&mut self) -> u16 {
        assert!(self.has_ready(), "RSTable: can't dequeue ready when there are no ready items");
        let index = (self.ready_queue_head % self.capacity as u64) as u16;
        let rs_ready_index = self.ready_queue[index as usize];

        self.ready_queue_head += 1;
        return rs_ready_index;
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
}

struct EU {
    index: u8,
    rs_index: u16,
    cycles_remaining: u8,
}

struct EUTable {
    capacity: u8,
    free_stack: Vec<u8>,
    array: Vec<EU>,
}

impl EUTable {
    fn new(capacity: u8) -> EUTable {
        let mut free_stack = Vec::with_capacity(capacity as usize);
        let mut array = Vec::with_capacity(capacity as usize);
        for i in 0..capacity {
            array.push(EU { index: i, cycles_remaining: 0, rs_index: 0 });
            free_stack.push(i);
        }

        EUTable {
            capacity,
            array,
            free_stack,
        }
    }

    fn has_free(&self) -> bool {
        return !self.free_stack.is_empty();
    }

    fn get(&self, reg: RegisterType) -> &EU {
        return self.array.get(reg as usize).unwrap();
    }

    fn get_mut(&mut self, eu_index: u8) -> &mut EU {
        return self.array.get_mut(eu_index as usize).unwrap();
    }

    fn allocate(&mut self) -> u8 {
        if let Some(last_element) = self.free_stack.pop() {
            return last_element;
        } else {
            panic!("No free PhysReg")
        }
    }

    fn deallocate(&mut self, eu_index: u8) {
        self.free_stack.push(eu_index);
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


#[derive(Clone, Copy, PartialEq)]
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
    result: WordType,
    rs_index: u16,
}

struct ROB {
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
                result: 0,
                rs_index: 0,
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

        //println!("next issued with success");

        let index = (self.issued % self.capacity as u64) as u16;
        self.issued += 1;
        return index;
    }

    fn head_has_executed(&self) -> bool {
        // todo: we should not passed issued
        // we should not pass the head
        if self.tail == self.head {
            return false;
        }

        let index = (self.head % self.capacity as u64) as u16;
        let rob_slot = &self.slots[index as usize];
        return rob_slot.state == ROBSlotState::EXECUTED;
    }

    fn next_executed(&mut self) -> u16 {
        assert!(self.head_has_executed(), "ROB: can't next_retire because there are no slots retired");

        let index = (self.head % self.capacity as u64) as u16;
        self.head += 1;
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
    eu_table: Rc<RefCell<EUTable>>,
    trace: bool,
    retire_n_wide: u8,
    dispatch_n_wide: u8,
    issue_n_wide: u8,
}

impl Backend {
    pub(crate) fn new(cpu_config: &CPUConfig,
                      instr_queue: Rc<RefCell<InstrQueue>>,
                      memory_subsystem: Rc<RefCell<MemorySubsystem>>,
                      arch_reg_file: Rc<RefCell<ArgRegFile>>) -> Backend {
        Backend {
            trace: cpu_config.trace,
            instr_queue,
            memory_subsystem,
            arch_reg_file,
            rs_table: Rc::new(RefCell::new(RSTable::new(cpu_config.rs_count))),
            phys_reg_file: Rc::new(RefCell::new(PhysRegFile::new(cpu_config.phys_reg_count))),
            rat: Rc::new(RefCell::new(RAT::new(cpu_config.phys_reg_count))),
            rob: Rc::new(RefCell::new(ROB::new(cpu_config.rob_capacity))),
            eu_table: Rc::new(RefCell::new(EUTable::new(cpu_config.eu_count))),
            retire_n_wide: cpu_config.retire_n_wide,
            dispatch_n_wide: cpu_config.dispatch_n_wide,
            issue_n_wide: cpu_config.issue_n_wide,
        }
    }

    pub(crate) fn do_cycle(&mut self) {
        self.cycle_retire();
        self.cycle_eu_table();
        self.cycle_dispatch();
        self.cycle_issue();
    }

    fn cycle_eu_table(&mut self) {
        let mut eu_table = self.eu_table.borrow_mut();
        let mut rs_table = self.rs_table.borrow_mut();
        let mut rob = self.rob.borrow_mut();
        let mut phys_reg_file = self.phys_reg_file.borrow_mut();
        let mut memory_subsystem = self.memory_subsystem.borrow_mut();

        for eu_index in 0..eu_table.capacity {
            let mut eu = eu_table.get_mut(eu_index);
            if eu.cycles_remaining == 0 {
                // eu is free, ignore it.
                continue;
            }
            eu.cycles_remaining -= 1;
            if eu.cycles_remaining > 0 {
                // the execution unit isn't finished with its work
                continue;
            }

            // it is the last cycle; so lets give this Eu some real work
            let rs_index = eu.rs_index;
            let mut rs = rs_table.get_mut(rs_index);

            let rob_index = rs.rob_slot_index;
            let mut rob_slot = rob.get_mut(rob_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            if self.trace {
                println!("Executing {}", instr);
            }

            let mut result: WordType = 0;
            match rs.opcode {
                Opcode::ADD => result = rs.source[0].union.get_constant() + rs.source[1].union.get_constant(),
                Opcode::SUB => result = rs.source[0].union.get_constant() - rs.source[1].union.get_constant(),
                Opcode::MUL => result = rs.source[0].union.get_constant() * rs.source[1].union.get_constant(),
                Opcode::DIV => result = rs.source[0].union.get_constant() / rs.source[1].union.get_constant(),
                Opcode::MOD => result = rs.source[0].union.get_constant() % rs.source[1].union.get_constant(),
                Opcode::INC => result = rs.source[0].union.get_constant() + 1,
                Opcode::DEC => result = rs.source[0].union.get_constant() - 1,
                Opcode::LOAD => result = memory_subsystem.memory[rs.source[0].union.get_memory_addr() as usize],
                Opcode::STORE => {}
                Opcode::NOP => {}
                Opcode::PRINTR => {
                    println!("PRINTR {}", rs.source[0].union.get_constant());
                }
            }

            let eu_index = eu.index;
            eu_table.deallocate(eu_index);

            match rs.sink.op_type {
                OpType::REGISTER => {
                    let phys_reg = rs.sink.union.get_register();
                    let phys_reg_entry = phys_reg_file.get_mut(phys_reg);
                    phys_reg_entry.has_value = true;
                    phys_reg_entry.value = result;

                    // // broad cast
                    // for k in 0..rs_table.count {
                    //     let rs = rs_table.get_mut(k);
                    //     if rs.state == RSState::FREE {
                    //         continue
                    //     }
                    //
                    //     let rob_slot_index =  rs.rob_slot_index;
                    //     let rob_slot = rob.get_mut(k);
                    //     if (rob_slot.state != ROBSlotState::ISSUED) {
                    //         continue;
                    //     }
                    //
                    //     let rs = rs_table.get_mut(rob_slot.rs_index);
                    //     for l in 0..rs.source_cnt {
                    //         let sink_op = &mut rs.source[l as usize];
                    //         if (sink_op.op_type == OpType::REGISTER && sink_op.union.get_register() == phys_reg) {
                    //             sink_op.op_type = OpType::VALUE;
                    //             sink_op.union = OpUnion::Constant(result);
                    //             rs.source_ready_cnt + 1;
                    //         }
                    //     }
                    //
                    //     if rs.source_cnt == rs.source_ready_cnt {
                    //         rs_table.enqueue_ready(rob_slot.rs_index);
                    //     }
                    // }
                }
                OpType::MEMORY => {
                    // a store to memory
                    memory_subsystem.sb.store(rs.sb_pos, rs.sink.union.get_memory_addr(), result);
                }
                OpType::CONSTANT => {
                    panic!("Constants can't be sinks.");
                }
                OpType::UNUSED => {}
            }

            rs.state = RSState::FREE;
            rs_table.deallocate(rs_index);

            rob_slot.result = result;
            rob_slot.state = ROBSlotState::EXECUTED;
        }
    }

    fn cycle_retire(&mut self) {
        let mut rob = self.rob.borrow_mut();
        let mut rat = self.rat.borrow_mut();
        let mut phys_reg_file = self.phys_reg_file.borrow_mut();
        let mut arch_reg_file = self.arch_reg_file.borrow_mut();

        for _ in 0..self.retire_n_wide {
            if !rob.head_has_executed() {
                break;
            }

            let rob_slot_index = rob.next_executed();
            let mut rob_slot = rob.get_mut(rob_slot_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            println!("Retiring {}", instr);

            if instr.sink.op_type == OpType::REGISTER {
                let arch_reg = instr.sink.union.get_register();
                let rat_entry = rat.get_mut(arch_reg);
                let phys_reg = rat_entry.phys_reg;

                rat_entry.valid = false;

                phys_reg_file.deallocate(phys_reg);

                arch_reg_file.set_value(arch_reg, rob_slot.result);
            }
        }
    }

    // the problem is with the dispatch
    fn cycle_dispatch(&mut self) {
        let mut rs_table = self.rs_table.borrow_mut();
        let mut rob = self.rob.borrow_mut();
        let mut eu_table = self.eu_table.borrow_mut();

        for _ in 0..self.dispatch_n_wide {
            if !rs_table.has_ready() || !eu_table.has_free() {
                break;
            }

            let rs_index = rs_table.deque_ready();
            let rs = rs_table.get_mut(rs_index);

            let rob_slot_index = rs.rob_slot_index;

            let rob_slot = rob.get_mut(rob_slot_index);
            rob_slot.state = ROBSlotState::DISPATCHED;

            let eu_index = eu_table.allocate();

            let mut eu = eu_table.get_mut(eu_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            eu.rs_index = rs_index;
            eu.cycles_remaining = instr.cycles;

            if self.trace {
                println!("Dispatched {}", instr);
            }
        }
    }

    fn cycle_issue(&mut self) {
        let mut rob = self.rob.borrow_mut();
        let mut rs_table = self.rs_table.borrow_mut();
        let mut instr_queue = self.instr_queue.borrow_mut();
        let mut phys_reg_file = self.phys_reg_file.borrow_mut();
        let mut rat = self.rat.borrow_mut();
        let arch_reg_file = self.arch_reg_file.borrow();
        let mut memory_subsystem = self.memory_subsystem.borrow_mut();

        // try to put as many instructions into the rob
        for _ in 0..self.issue_n_wide {
            if instr_queue.is_empty() || !rob.has_space() {
                break;
            }

            let instr = instr_queue.peek();

            let mut rb_slot_index = Option::None;

            if instr.sink.op_type == OpType::MEMORY {
                if !memory_subsystem.sb.has_space() {
                    return;
                }
                rb_slot_index = Some(memory_subsystem.sb.allocate());
            }

            instr_queue.dequeue();


            let rob_slot_index = rob.allocate();
            let rob_slot = rob.get_mut(rob_slot_index);

            if self.trace {
                println!("Issued {}", instr);
            }

            rob_slot.rb_slot_index = rb_slot_index;
            rob_slot.state = ROBSlotState::DISPATCHED;
            rob_slot.instr = Some(instr);
        }

        // try to put as many instructions from the rob, into reservation stations
        for _ in 0..self.issue_n_wide {
            if !rob.has_issued() || !rs_table.has_free() {
                break;
            }

            let rob_slot_index = rob.next_issued();
            let mut rob_slot = rob.get_mut(rob_slot_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);
            let rs_index = rs_table.allocate();
            let mut rs = rs_table.get_mut(rs_index);

            if self.trace {
                println!("Issued found RS {}", instr);
            }
            rob_slot.state = ROBSlotState::ISSUED;
            rob_slot.result = 0;
            rob_slot.rs_index = rs_index;

            rs.rob_slot_index = rob_slot_index;
            //println!("cycle_issue: rob_slot_index  {}", rs.rob_slot_index);
            rs.opcode = instr.opcode;
            rs.state = RSState::BUSY;

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
                    op_rs.union = OpUnion::Register(phys_reg);
                }
                OpType::MEMORY => {
                    rs.sink = *op_instr;
                    // todo: not handling a full sb.
                    // since the instructions are issued in program order, a slot is allocated in the
                    // sb in program order. And since sb will commit to the coherent cache
                    // (in this case directly to memory), the stores will become visible
                    // in program order.
                    rs.sb_pos = memory_subsystem.sb.allocate();
                }
                OpType::CONSTANT => {
                    panic!("Can't have a value as sink {}", op_rs)
                }
                OpType::UNUSED => {}
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
                            let phys_reg_entry = phys_reg_file.get(rat_entry.phys_reg);
                            if phys_reg_entry.has_value {
                                //we got lucky, there is a value in the physical register.
                                let value = phys_reg_entry.value;
                                rs_op.op_type = OpType::CONSTANT;
                                rs_op.union = OpUnion::Constant(value);
                                rs.source_ready_cnt += 1;
                            } else {
                                // cdb broadcast will update
                                rs_op.op_type = OpType::REGISTER;
                                rs_op.union = OpUnion::Register(rat_entry.phys_reg);
                            }
                        } else {
                            let value = arch_reg_file.get_value(arch_reg);
                            rs_op.op_type = OpType::CONSTANT;
                            rs_op.union = OpUnion::Constant(value);
                            rs.source_ready_cnt += 1;
                        }
                    }
                    OpType::MEMORY | OpType::CONSTANT => {
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
