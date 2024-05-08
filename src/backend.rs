use std::rc::Rc;
use std::cell::RefCell;
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

    fn get(&self, sb_index: u16) -> & RS {
        return &self.array[sb_index as usize];
    }

    fn get_mut(&mut self, sb_index: u16) -> & mut RS {
        return self.array.get_mut(sb_index as usize).unwrap();
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

    fn deque_ready(&mut self) -> & mut RS {
        // if !self.has_ready() {
        //     panic!();
        // }

        unsafe {
            let index = (self.ready_queue_head % self.count as u64) as usize;
            let rs_ptr = self.array.get_mut(index as usize).unwrap() as *mut RS;
            let rs_ref = &mut *rs_ptr;
            return rs_ref;
        }
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

    fn deallocate(&mut self, rs: & mut RS) {
        rs.state = RSState::FREE;
        self.free_stack.push(rs.index);
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

pub(crate) struct Backend<'a> {
    instr_queue: Rc<RefCell<InstrQueue<'a>>>,
    rs_table: Rc<RefCell<RSTable>>,
    phys_reg_file: Rc<RefCell<PhysRegFile>>,
    arch_reg_file: Rc<RefCell<ArgRegFile>>,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    rat: Rc<RefCell<RAT>>,
}

impl<'a> Backend<'a> {
    pub(crate) fn new(cpu_config: &'a CPUConfig,
                      instr_queue: Rc<RefCell<InstrQueue<'a>>>,
                      memory_subsystem: Rc<RefCell<MemorySubsystem>>,
                      arch_reg_file: Rc<RefCell<ArgRegFile>>) -> Backend<'a> {
        Backend {
            instr_queue,
            memory_subsystem,
            arch_reg_file,
            rs_table: Rc::new(RefCell::new(RSTable::new(cpu_config.rs_count))),
            phys_reg_file: Rc::new(RefCell::new(PhysRegFile::new(cpu_config.phys_reg_count))),
            rat: Rc::new(RefCell::new(RAT::new(cpu_config.phys_reg_count))),
        }
    }

    pub(crate) fn do_cycle(&mut self) {
        self.cycle_retire();
        self.cycle_dispatch();
        self.cycle_issue();
    }

    fn cycle_retire(&mut self) {}

    fn cycle_dispatch(&mut self) {
        while self.rs_table.borrow().has_ready() {
            let rs;
            {
                let mut rs_table = self.rs_table.borrow_mut();
                rs = rs_table.deque_ready();
            }
            // Process the dequeued RS
            // println!("dispatch {}", rs);
            //  let junk = &mut RS::new(1);
            //  self.rs_table.borrow_mut().deallocate(junk);
        }
    }

    fn cycle_issue(&mut self) {
        loop {
            if self.instr_queue.borrow().is_empty() {
                // there is no instructon in the instruction queue
                return;
            }

            if !self.rs_table.borrow().has_free() {
                // there are no free reservation stations
                return;
            }

            let instr = self.instr_queue.borrow().peek();

            let mut rs_table = self.rs_table.borrow_mut();
            let rs_index = rs_table.allocate();
            let rs = &mut rs_table.get_mut(rs_index);
            println!("Issued {}", instr);

            rs.state = RSState::BUSY;
            rs.sink_available = instr.sink_available;
            if instr.sink_available {
                let op_instr = &instr.sink;
                let op_rs = &mut rs.sink;

                match op_instr.op_type {
                    OpType::REGISTER => {
                        let arch_reg = op_instr.union.get_register();
                        let mut phys_reg_file = self.phys_reg_file.borrow_mut();
                        let phys_reg = phys_reg_file.allocate();
                        let mut rat = self.rat.borrow_mut();
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
                        let rat = self.rat.borrow();
                        let rat_entry = rat.get(arch_reg);
                        if rat_entry.valid {
                            rs_op.op_type = OpType::REGISTER;
                            rs_op.union = OpUnion::Register(rat_entry.phys_reg);
                        } else {
                            let value = self.arch_reg_file.borrow().get_value(arch_reg);
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
                rs_table.enqueue_ready(rs.index);
            }

            self.instr_queue.borrow_mut().dequeue();
        }
    }
}
