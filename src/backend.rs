use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use std::fmt::Display;
use crate::cpu::CPUConfig;
use crate::instructions::{InstrQueue, mnemonic, Opcode, Operand, OpType, OpUnion, WordType};
use crate::memory_subsystem::MemorySubsystem;

struct ArgReg {
    value: WordType,
}

struct ArgRegFile {
    registers: Vec<ArgReg>,
}

impl ArgRegFile {
    fn new(rs_count: u16) -> ArgRegFile {
        let mut array = Vec::with_capacity(rs_count as usize);
        for i in 0..rs_count {
            array.push(ArgReg { value: 0 });
        }

        ArgRegFile { registers: array }
    }
}

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
    fn new(rs_count: u16) -> PhysRegFile {
        let mut free_stack = Vec::with_capacity(rs_count as usize);
        let mut array = Vec::with_capacity(rs_count as usize);
        for i in 0..rs_count {
            array.push(PhysReg { value: 0, has_value: false, index: i });
            free_stack.push(i);
        }

        PhysRegFile {
            count: rs_count,
            registers: array,
            free_stack,
        }
    }

    fn has_free(&self) -> bool {
        return !self.free_stack.is_empty();
    }

    fn allocate(&mut self) -> &mut PhysReg {
        if let Some(last_element) = self.free_stack.pop() {
            unsafe {
                let phys_reg_ptr = self.registers.get_mut(last_element as usize).unwrap() as *mut PhysReg;
                let phys_reg = &mut *phys_reg_ptr;
                phys_reg.has_value = false;
                return phys_reg;
            }
        } else {
            panic!("No free PhysReg")
        }
    }

    fn deallocate(&mut self, rs: &mut PhysReg) {
        self.free_stack.push(rs.index);
    }

    //pub
}

enum RS_State {
    FREE,
    BUSY,
}

struct RS {
    index: u16,
    opcode: Opcode,
    state: RS_State,
    sink_cnt: u8,
    sink: [Operand; crate::instructions::MAX_SINK_COUNT as usize],
    source_cnt: u8,
    source: [Operand; crate::instructions::MAX_SOURCE_COUNT as usize],
}

impl RS {
    fn new(index: u16) -> Self {
        Self {
            index,
            opcode: Opcode::NOP,
            state: RS_State::FREE,
            source_cnt: 0,
            source: [
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused },
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
            ],
            sink_cnt: 0,
            sink: [
                Operand { op_type: OpType::UNUSED, union: OpUnion::Unused }
            ],
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

        for k in 0..self.sink_cnt {
            write!(f, " {}", self.sink[k as usize])?;
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
    fn new(rs_count: u16) -> RSTable {
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

    fn enqueue_ready(&mut self, rs: &RS) {
        let index = (self.ready_queue_tail % self.count as u64) as usize;
        self.ready_queue[index] = rs.index;
        self.ready_queue_tail += 1;
    }

    // todo: has_ready/dequeue_ready can be simplified by using an Option
    fn has_ready(&self) -> bool {
        return self.ready_queue_head != self.ready_queue_tail;
    }

    fn deque_ready(&mut self) -> &mut RS {
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

    fn allocate(&mut self) -> &mut RS {
        if let Some(last_element) = self.free_stack.pop() {
            unsafe {
                let rs_ptr = self.array.get_mut(last_element as usize).unwrap() as *mut RS;
                let rs_ref = &mut *rs_ptr;
                rs_ref.state = RS_State::BUSY;
                return rs_ref;
            }
        } else {
            panic!("No free RS")
        }
    }

    fn deallocate(&mut self, rs: &mut RS) {
        rs.state = RS_State::FREE;
        self.free_stack.push(rs.index);
    }

    //pub
}

pub(crate) struct Backend<'a> {
    instr_queue: Rc<RefCell<InstrQueue<'a>>>,
    rs_table: RSTable,
    phys_reg_file: PhysRegFile,
    arch_reg_file: ArgRegFile,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
}

impl<'a> Backend<'a> {
    pub(crate) fn new(cpu_config: &'a CPUConfig,
                      instr_queue: Rc<RefCell<InstrQueue<'a>>>,
                      memory_subsystem: Rc<RefCell<MemorySubsystem>>) -> Backend<'a> {
        Backend {
            instr_queue,
            memory_subsystem,
            rs_table: RSTable::new(cpu_config.rs_count),
            phys_reg_file: PhysRegFile::new(cpu_config.phys_reg_count),
            arch_reg_file: ArgRegFile::new(cpu_config.arch_reg_count),
        }
    }

    pub(crate) fn cycle(&mut self) {
        self.cycle_issue();
    }

    fn cycle_dispatch(&mut self) {
        while self.rs_table.has_ready() {
            //
            // let rs = self.rs_table.deque_ready();
            //
            // println!("dispatch {}", rs);
            //
            // self.rs_table.deallocate(rs);
        }
    }

    fn cycle_issue(&mut self) {
        loop {
            if !self.rs_table.has_free() {
                println!("Backend: No free RS");
                // There are no free RS, we are done
                return;
            }

            match self.instr_queue.borrow_mut().dequeue() {
                None => {
                    // there are no available instructions, so we aredone
                    return;
                }
                Some(instr) => {
                    let rs = self.rs_table.allocate();
                    println!("Issued {}", instr);

                    // todo: use right opcode
                    rs.opcode = Opcode::NOP;
                    rs.source_cnt = 0;
                    rs.sink_cnt = 0;
                    //self.rs_table.enqueue_ready(rs);
                }
            }
        }
    }
}
