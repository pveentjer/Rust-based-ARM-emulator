use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::execution_unit::EUTable;
use crate::backend::physical_register::PhysRegFile;
use crate::backend::register_alias_table::RAT;
use crate::backend::reorder_buffer::{ROB, ROBSlotState};
use crate::backend::reservation_station::{RSState, RSTable};
use crate::cpu::{ArgRegFile, CPUConfig, PerfCounters, Trace};
use crate::frontend::frontend::FrontendControl;
use crate::instructions::instructions::{Instr, InstrQueue, Opcode, Operand, RegisterType, WordType};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;

struct CDBBroadcast {
    phys_reg: RegisterType,
    value: WordType,
}

pub struct Backend {
    instr_queue: Rc<RefCell<InstrQueue>>,
    arch_reg_file: Rc<RefCell<ArgRegFile>>,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    frontend_control: Rc<RefCell<FrontendControl>>,
    rs_table: RSTable,
    phys_reg_file: PhysRegFile,
    rat: RAT,
    rob: ROB,
    eu_table: EUTable,
    trace: Trace,
    retire_n_wide: u8,
    dispatch_n_wide: u8,
    issue_n_wide: u8,
    cdb_broadcast_buffer: Vec<CDBBroadcast>,
    stack: Vec<WordType>,
    stack_capacity: u32,
    pub(crate) exit: bool,
    perf_counters: Rc<RefCell<PerfCounters>>,
}

impl Backend {
    pub(crate) fn new(cpu_config: &CPUConfig,
                      instr_queue: Rc<RefCell<InstrQueue>>,
                      memory_subsystem: Rc<RefCell<MemorySubsystem>>,
                      arch_reg_file: Rc<RefCell<ArgRegFile>>,
                      frontend_control: Rc<RefCell<FrontendControl>>,
                      perf_counters: Rc<RefCell<PerfCounters>>) -> Backend {
        let mut stack = Vec::with_capacity(cpu_config.stack_capacity as usize);
        for _ in 0..cpu_config.stack_capacity {
            stack.push(0);
        }

        Backend {
            trace: cpu_config.trace.clone(),
            instr_queue,
            memory_subsystem,
            arch_reg_file,
            rs_table: RSTable::new(cpu_config.rs_count),
            phys_reg_file: PhysRegFile::new(cpu_config.phys_reg_count),
            rat: RAT::new(cpu_config.phys_reg_count),
            rob: ROB::new(cpu_config.rob_capacity),
            eu_table: EUTable::new(cpu_config.eu_count),
            retire_n_wide: cpu_config.retire_n_wide,
            dispatch_n_wide: cpu_config.dispatch_n_wide,
            issue_n_wide: cpu_config.issue_n_wide,
            cdb_broadcast_buffer: Vec::with_capacity(cpu_config.eu_count as usize),
            frontend_control,
            stack,
            stack_capacity: cpu_config.stack_capacity,
            exit: false,
            perf_counters,
        }
    }

    pub(crate) fn do_cycle(&mut self) {
        self.cycle_retire();
        self.cycle_eu_table();
        self.cdb_broadcast();
        self.cycle_dispatch();
        self.cycle_issue();
    }

    fn cycle_eu_table(&mut self) {
        let mut memory_subsystem = self.memory_subsystem.borrow_mut();
        let mut perf_monitors = self.perf_counters.borrow_mut();

        for eu_index in 0..self.eu_table.capacity {
            let mut eu = self.eu_table.get_mut(eu_index);
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
            let mut rs = self.rs_table.get_mut(rs_index);

            let rob_index = rs.rob_slot_index;
            let mut rob_slot = self.rob.get_mut(rob_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            if self.trace.execute {
                println!("Executing {}", instr);
            }

            match rs.opcode {
                Opcode::NOP => {}
                Opcode::ADD => rob_slot.result.push(rs.source[0].get_constant() + rs.source[1].get_constant()),
                Opcode::SUB => rob_slot.result.push(rs.source[0].get_constant() - rs.source[1].get_constant()),
                Opcode::MUL => rob_slot.result.push(rs.source[0].get_constant() * rs.source[1].get_constant()),
                Opcode::SDIV => rob_slot.result.push(rs.source[0].get_constant() / rs.source[1].get_constant()),
                Opcode::NEG => rob_slot.result.push(-rs.source[0].get_constant()),
                Opcode::AND => rob_slot.result.push(rs.source[0].get_constant() & rs.source[1].get_constant()),
                Opcode::ORR => rob_slot.result.push(rs.source[0].get_constant() | rs.source[1].get_constant()),
                Opcode::EOR => rob_slot.result.push(rs.source[0].get_constant() ^ rs.source[1].get_constant()),
                Opcode::NOT => rob_slot.result.push(!rs.source[0].get_constant()),
                Opcode::MOV => rob_slot.result.push(rs.source[0].get_constant()),
                Opcode::LDR => rob_slot.result.push(memory_subsystem.memory[rs.source[0].get_memory_addr() as usize]),
                Opcode::STR => rob_slot.result.push(rs.source[0].get_constant()),
                Opcode::PRINTR => {
                    println!("PRINTR R{}={}", instr.source[0].get_register(), rs.source[0].get_constant());
                }
                Opcode::JNZ | Opcode::JZ => {
                    let mut frontend_control = self.frontend_control.borrow_mut();
                    let value = rs.source[0].get_constant();
                    let take_branch = match rs.opcode {
                        Opcode::JNZ => value != 0,
                        Opcode::JZ => value == 0,
                        _ => unreachable!(),
                    };

                    if take_branch {
                        frontend_control.ip_next_fetch = rs.source[1].get_code_address() as i64;
                    } else {
                        frontend_control.ip_next_fetch += 1;
                    }
                    frontend_control.halted = false;
                }
                Opcode::CALL => {
                    let mut frontend_control = self.frontend_control.borrow_mut();

                    let rsp_value = rs.source[0].get_constant();
                    let new_rsp_value = rsp_value + 1;
                    let code_address = rs.source[1].get_code_address();

                    // on the stack we store the current ip
                    self.stack[rsp_value as usize] = frontend_control.ip_next_fetch;
                    // update the rsp
                    rob_slot.result.push(new_rsp_value);

                    frontend_control.ip_next_fetch = code_address as i64;
                    frontend_control.halted = false;
                }
                Opcode::RET => {
                    let mut frontend_control = self.frontend_control.borrow_mut();

                    let rsp_value = rs.source[0].get_constant();

                    let new_rsp_value = rsp_value - 1;
                    let ip_next_fetch = self.stack[new_rsp_value as usize];

                    // update the rsp
                    rob_slot.result.push(new_rsp_value);

                    // because CALL is a control, the ip_next_fetch is still pointing to the CALL and so we need to bump it manually
                    frontend_control.ip_next_fetch = ip_next_fetch + 1;
                    frontend_control.halted = false;
                }
                Opcode::PUSH => {
                    let value = rs.source[0].get_constant();
                    let rsp_value = rs.source[1].get_constant();

                    if rsp_value as usize == self.stack_capacity as usize {
                        panic!("Ran out of stack");
                    }

                    self.stack[rsp_value as usize] = value;
                    rob_slot.result.push(rsp_value + 1);
                }
                Opcode::POP => {
                    let rsp_value = (rs.source[0].get_constant() - 1) as WordType;

                    rob_slot.result.push(self.stack[rsp_value as usize]);
                    rob_slot.result.push(rsp_value);
                }
                Opcode::EXIT => {}
            }

            let eu_index = eu.index;
            self.eu_table.deallocate(eu_index);

            for sink_index in 0..rs.sink_cnt {
                let sink = rs.sink[sink_index as usize];
                match sink {
                    Operand::Register(phys_reg) => {
                        let phys_reg_entry = self.phys_reg_file.get_mut(phys_reg);
                        phys_reg_entry.has_value = true;
                        let result = rob_slot.result[sink_index as usize];
                        phys_reg_entry.value = result;
                        self.cdb_broadcast_buffer.push(CDBBroadcast { phys_reg, value: result });
                    }
                    Operand::Memory(addr) => {
                        let result = rob_slot.result[sink_index as usize];
                        // a store to memory
                        memory_subsystem.sb.store(rs.sb_pos, addr, result);
                    }
                    Operand::Immediate(_) | Operand::Code(_) | Operand::Unused => panic!("Illegal sink {:?}", sink),
                }
            }

            rs.state = RSState::FREE;
            self.rs_table.deallocate(rs_index);

            rob_slot.state = ROBSlotState::EXECUTED;
            perf_monitors.execute_cnt += 1;
        }
    }

    fn cdb_broadcast(&mut self) {
        let rs_table_capacity = self.rs_table.capacity;

        for req in &mut *self.cdb_broadcast_buffer {
            // Iterate over all RS and replace every matching physical register, by the value
            for rs_index in 0..rs_table_capacity {
                let rs = self.rs_table.get_mut(rs_index);
                if rs.state == RSState::FREE {
                    continue;
                }

                let rob_slot_index = rs.rob_slot_index;
                let rob_slot = self.rob.get_mut(rob_slot_index);
                if rob_slot.state != ROBSlotState::ISSUED {
                    continue;
                }

                let rs = self.rs_table.get_mut(rob_slot.rs_index);
                for source_index in 0..rs.source_cnt as usize {
                    let source_rs = &mut rs.source[source_index];
                    if let Operand::Register(phys_reg) = source_rs {
                        if *phys_reg == req.phys_reg {
                            rs.source[source_index] = Operand::Immediate(req.value);
                            rs.source_ready_cnt += 1;
                        }
                    }
                }

                if rs.source_cnt == rs.source_ready_cnt {
                    rob_slot.state = ROBSlotState::DISPATCHED;
                    self.rs_table.enqueue_ready(rob_slot.rs_index);
                }
            }
        }

        self.cdb_broadcast_buffer.clear();
    }

    fn cycle_retire(&mut self) {
        let mut arch_reg_file = self.arch_reg_file.borrow_mut();
        let mut perf_monitors = self.perf_counters.borrow_mut();

        for _ in 0..self.retire_n_wide {
            if !self.rob.head_has_executed() {
                break;
            }

            let rob_slot_index = self.rob.next_executed();
            let mut rob_slot = self.rob.get_mut(rob_slot_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            if instr.opcode == Opcode::EXIT {
                self.exit = true;
            }

            if self.trace.retire {
                println!("Retiring {}", instr);
            }

            perf_monitors.retire_cnt += 1;

            for sink_index in 0..instr.sink_cnt as usize {
                let sink = instr.sink[sink_index];
                if let Operand::Register(arch_reg) = sink {
                    let rat_entry = self.rat.get_mut(arch_reg);
                    let rat_phys_reg = rat_entry.phys_reg;
                    let rs_phys_reg = rob_slot.sink[sink_index].get_register();

                    // only when the physical register on the rat is the same as te physical register used for that
                    // instruction, the rat entry should be invalidated
                    if rat_phys_reg == rs_phys_reg {
                        rat_entry.valid = false;
                    }

                    self.phys_reg_file.get_mut(rs_phys_reg).has_value = false;
                    self.phys_reg_file.deallocate(rs_phys_reg);
                    arch_reg_file.set_value(arch_reg, rob_slot.result[sink_index]);
                }
            }
        }
    }

    fn cycle_dispatch(&mut self) {
        let mut perf_monitors = self.perf_counters.borrow_mut();

        for _ in 0..self.dispatch_n_wide {
            if !self.rs_table.has_ready() || !self.eu_table.has_free() {
                break;
            }

            let rs_index = self.rs_table.deque_ready();
            let rs = self.rs_table.get_mut(rs_index);

            let rob_slot_index = rs.rob_slot_index;

            let rob_slot = self.rob.get_mut(rob_slot_index);
            rob_slot.state = ROBSlotState::DISPATCHED;

            let eu_index = self.eu_table.allocate();

            let mut eu = self.eu_table.get_mut(eu_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            eu.rs_index = rs_index;
            eu.cycles_remaining = instr.cycles;

            if self.trace.dispatch {
                println!("Dispatched [{}]", instr);
            }

            perf_monitors.dispatch_cnt += 1;
        }
    }

    fn cycle_issue(&mut self) {
        let mut perf_monitors = self.perf_counters.borrow_mut();

        let mut instr_queue = self.instr_queue.borrow_mut();
        let arch_reg_file = self.arch_reg_file.borrow();
        let mut memory_subsystem = self.memory_subsystem.borrow_mut();

        // try to put as many instructions into the rob
        for _ in 0..self.issue_n_wide {
            if instr_queue.is_empty() || !self.rob.has_space() {
                break;
            }

            let instr = instr_queue.peek();

            instr_queue.dequeue();


            let rob_slot_index = self.rob.allocate();
            let rob_slot = self.rob.get_mut(rob_slot_index);

            if self.trace.issue {
                println!("issue: Issued [{}]", instr);
            }

            rob_slot.state = ROBSlotState::ISSUED;
            rob_slot.instr = Some(instr);

            perf_monitors.issue_cnt += 1;
        }

        // try to put as many instructions from the rob, into reservation stations
        for _ in 0..self.issue_n_wide {
            if !self.rob.has_issued() || !self.rs_table.has_free() {
                break;
            }

            let rob_slot_index = self.rob.next_issued();
            let mut rob_slot = self.rob.get_mut(rob_slot_index);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            if instr.mem_stores > 0 && !memory_subsystem.sb.has_space() {
                // we can't allocate a slot in the store buffer, we are done
                break;
            }

            let rs_index = self.rs_table.allocate();
            let mut rs = self.rs_table.get_mut(rs_index);

            if self.trace.issue {
                println!("issue: Issued found RS [{}]", instr);
            }
            rob_slot.state = ROBSlotState::ISSUED;
            rob_slot.result.clear();
            rob_slot.rs_index = rs_index;

            rs.rob_slot_index = rob_slot_index;
            //println!("cycle_issue: rob_slot_index  {}", rs.rob_slot_index);
            rs.opcode = instr.opcode;
            rs.state = RSState::BUSY;

            rs.source_cnt = instr.source_cnt;
            rs.source_ready_cnt = 0;

            for source_index in 0..instr.source_cnt as usize {
                let instr_source = &instr.source[source_index];
                let rs_source = &rs.source[source_index];
                match instr_source {
                    Operand::Register(arch_reg) => {
                        let rat_entry = self.rat.get(*arch_reg);
                        if rat_entry.valid {
                            let phys_reg_entry = self.phys_reg_file.get(rat_entry.phys_reg);
                            if phys_reg_entry.has_value {

                                //we got lucky, there is a value in the physical register.
                                rs.source[source_index] = Operand::Immediate(phys_reg_entry.value);
                                rs.source_ready_cnt += 1;
                            } else {
                                // cdb broadcast will update
                                rs.source[source_index] = Operand::Register(rat_entry.phys_reg);
                            }
                        } else {
                            let value = arch_reg_file.get_value(*arch_reg);
                            rs.source[source_index] = Operand::Immediate(value);
                            rs.source_ready_cnt += 1;
                        }
                    }
                    Operand::Memory(_) | Operand::Immediate(_) | Operand::Code(_) => {
                        rs.source[source_index] = *instr_source;
                        rs.source_ready_cnt += 1;
                    }
                    Operand::Unused =>
                        panic!("Illegal source {:?}", rs_source)
                }
            }

            rs.sink_cnt = instr.sink_cnt;
            for sink_index in 0..instr.sink_cnt as usize {
                let instr_sink = instr.sink[sink_index];
                match instr_sink {
                    Operand::Register(arch_reg) => {
                        let phys_reg = self.phys_reg_file.allocate();
                        // update the RAT entry to point to the newest phys_reg
                        let rat_entry = self.rat.get_mut(arch_reg);
                        rat_entry.phys_reg = phys_reg;
                        rat_entry.valid = true;

                        // Update the sink on the RS.
                        rs.sink[sink_index] = Operand::Register(phys_reg);
                    }
                    Operand::Memory(_) => {
                        rs.sink[sink_index] = instr_sink;
                        // since the instructions are issued in program order, a slot is allocated in the
                        // sb in program order. And since sb will commit to the coherent cache
                        // (in this case directly to memory), the stores will become visible
                        // in program order.
                        rs.sb_pos = memory_subsystem.sb.allocate();
                    }
                    Operand::Unused | Operand::Immediate(_) | Operand::Code(_) => {
                        panic!("Illegal sink {:?}", instr_sink)
                    }
                }
            }
            rob_slot.sink = rs.sink;

            if rs.source_ready_cnt == rs.source_cnt {
                rob_slot.state = ROBSlotState::DISPATCHED;
                self.rs_table.enqueue_ready(rs_index);
            }
        }
    }
}