use std::cell::RefCell;

use std::rc::Rc;

use crate::backend::execution_unit::{EUState, EUTable};
use crate::backend::physical_register::PhysRegFile;
use crate::backend::register_alias_table::RAT;
use crate::backend::reorder_buffer::{ROB, ROBSlotState};
use crate::backend::reservation_station::{RSState, RSTable};
use crate::cpu::{ArgRegFile, CARRY_FLAG, CPUConfig, NEGATIVE_FLAG, OVERFLOW_FLAG, PC, PerfCounters, Trace, ZERO_FLAG};
use crate::frontend::frontend::FrontendControl;
use crate::instructions::instructions::{Instr, InstrQueue, Opcode, Operand, RegisterType, WordType};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;

struct CDBBroadcast {
    phys_reg: RegisterType,
    value: WordType,
}

pub(crate) struct Backend {
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
            exit: false,
            perf_counters,
        }
    }

    pub(crate) fn do_cycle(&mut self) {
        self.cycle_retire();
        self.cycle_eu_table();
        debug_assert!(self.cdb_broadcast_buffer.is_empty());
        self.cycle_dispatch();
        self.cycle_rs_allocation();
        self.cycle_issue();
    }

    // issues as many instructions from the instruction queue into the rob as possible.
    fn cycle_issue(&mut self) {
        let mut perf_monitors = self.perf_counters.borrow_mut();
        let mut instr_queue = self.instr_queue.borrow_mut();

        // try to put as many instructions into the rob
        for _ in 0..self.issue_n_wide {
            // println!("cycle_issue: instr_queue.isempty: {}, self.rob.has_space: {}", instr_queue.is_empty(), self.rob.has_space());

            if instr_queue.is_empty() || !self.rob.has_space() {
                break;
            }

            // todo: register renaming should be done here.

            let instr_queue_head_index = instr_queue.head_index();
            let instr_queue_slot = instr_queue.get_mut(instr_queue_head_index);

            let branch_target_predicted = instr_queue_slot.branch_target_predicted;
            let instr = Rc::clone(&instr_queue_slot.instr);

            // If needed, synchronize of the sb being empty
            if instr.sb_sync() && self.memory_subsystem.borrow().sb.size() > 0 {
                return;
            }

            // If needed, synchronize on the rob being empty
            if instr.rob_sync() && self.rob.size() > 0 {
                return;
            }

            let rob_slot_index = self.rob.allocate();
            let rob_slot = self.rob.get_mut(rob_slot_index);

            if self.trace.issue {
                println!("Issued [{}]", instr);
            }

            rob_slot.pc = instr_queue_slot.pc;
            rob_slot.state = ROBSlotState::ISSUED;
            rob_slot.instr = Some(instr);
            rob_slot.branch_target_predicted = branch_target_predicted;
            self.rob.seq_issued += 1;
            perf_monitors.issue_cnt += 1;

            instr_queue.head_bump();
        }
    }

    // For any rob entry that doesn't have a reservation station, try to look up a rs.
    fn cycle_rs_allocation(&mut self) {
        let arch_reg_file = self.arch_reg_file.borrow();
        let mut memory_subsystem = self.memory_subsystem.borrow_mut();

        for _ in 0..self.issue_n_wide {
            if self.rob.seq_rs_allocated == self.rob.seq_issued || !self.rs_table.has_idle() {
                break;
            }

            let rob_slot_index = self.rob.to_index(self.rob.seq_rs_allocated);
            let rob_slot = self.rob.get_mut(rob_slot_index);

            debug_assert!(rob_slot.state == ROBSlotState::ISSUED);
            debug_assert!(rob_slot.eu_index.is_none());
            debug_assert!(rob_slot.rs_index.is_none());

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            if instr.mem_stores > 0 && !memory_subsystem.sb.has_space() {
                // we can't allocate a slot in the store buffer, we are done
                break;
            }

            let rs_index = self.rs_table.allocate();
            let rs = self.rs_table.get_mut(rs_index);
            debug_assert!(rs.state == RSState::BUSY);

            rob_slot.rs_index = Some(rs_index);

            rs.rob_slot_index = Some(rob_slot_index);
            rs.opcode = instr.opcode;
            rs.source_cnt = instr.source_cnt;

            // Register renaming of the source operands
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

            // Register renaming of the sink operands.
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
                        rob_slot.sb_pos = Some(memory_subsystem.sb.allocate());
                    }
                    Operand::Unused | Operand::Immediate(_) | Operand::Code(_) => {
                        panic!("Illegal sink {:?}", instr_sink)
                    }
                }
            }
            rob_slot.sink = rs.sink;

            if rs.source_ready_cnt == rs.source_cnt {
                self.rs_table.enqueue_ready(rs_index);
            }

            if self.trace.allocate_rs {
                println!("Allocate RS [{}]", instr);
            }

            self.rob.seq_rs_allocated += 1;
        }
    }

    fn cycle_dispatch(&mut self) {
        let mut perf_monitors = self.perf_counters.borrow_mut();

        for _ in 0..self.dispatch_n_wide {
            if !self.rs_table.has_ready() || !self.eu_table.has_idle() {
                break;
            }

            let rs_index = self.rs_table.deque_ready();
            //println!("Dispatch rs_index {}", rs_index);

            let rs = self.rs_table.get_mut(rs_index);
            debug_assert!(rs.state == RSState::BUSY);

            //println!("Cycle dispatch: opcode {:?}", rs.opcode);

            //println!("RS {:?}", rs.state);

            let rob_slot_index = rs.rob_slot_index.unwrap();
            let rob_slot = self.rob.get_mut(rob_slot_index);

            let eu_index = self.eu_table.allocate();
            let eu = self.eu_table.get_mut(eu_index);
            debug_assert!(eu.state == EUState::BUSY);

            let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
            let instr = Rc::clone(&rc);

            eu.rs_index = Some(rs_index);
            eu.cycles_remaining = instr.cycles;

            rob_slot.state = ROBSlotState::DISPATCHED;
            rob_slot.eu_index = Some(eu_index);

            if self.trace.dispatch {
                println!("Dispatched [{}]", instr);
            }
            perf_monitors.dispatch_cnt += 1;
        }
    }

    fn cycle_eu_table(&mut self) {
        {
            let mut memory_subsystem = self.memory_subsystem.borrow_mut();
            let mut perf_monitors = self.perf_counters.borrow_mut();

            // todo: we should only iterate over the used execution units.
            for eu_index in 0..self.eu_table.capacity {
                let eu = self.eu_table.get_mut(eu_index);

                if eu.state == EUState::IDLE {
                    continue;
                }

                let rs_index = eu.rs_index.unwrap();

                let rs = self.rs_table.get_mut(rs_index);
                debug_assert!(rs.state == RSState::BUSY);

                let rob_index = rs.rob_slot_index.unwrap();
                let rob_slot = self.rob.get_mut(rob_index);
                debug_assert!(rob_slot.state == ROBSlotState::DISPATCHED,
                              "rob_slot is not in dispatched state, but in {:?}, rs_index={}", rob_slot.state, rs_index);
                debug_assert!(rob_slot.rs_index.is_some());
                debug_assert!(rob_slot.eu_index.is_some());

                eu.cycles_remaining -= 1;

                if eu.cycles_remaining > 0 {
                    // the execution unit isn't finished with its work
                    continue;
                }

                // it is the last cycle; so lets give this Eu some real work
                let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
                let instr = Rc::clone(&rc);

                if self.trace.execute {
                    println!("Executing {}", instr);
                }

                match rs.opcode {
                    Opcode::NOP => {}
                    Opcode::ADD => rob_slot.result.push(rs.source[0].get_immediate() + rs.source[1].get_immediate()),
                    Opcode::SUB => rob_slot.result.push(rs.source[0].get_immediate() - rs.source[1].get_immediate()),
                    Opcode::MUL => rob_slot.result.push(rs.source[0].get_immediate() * rs.source[1].get_immediate()),
                    Opcode::SDIV => rob_slot.result.push(rs.source[0].get_immediate() / rs.source[1].get_immediate()),
                    Opcode::NEG => rob_slot.result.push(-rs.source[0].get_immediate()),
                    Opcode::AND => rob_slot.result.push(rs.source[0].get_immediate() & rs.source[1].get_immediate()),
                    Opcode::MOV => rob_slot.result.push(rs.source[0].get_immediate()),
                    Opcode::ADR => {
                        //todo
                    }
                    Opcode::ORR => rob_slot.result.push(rs.source[0].get_immediate() | rs.source[1].get_immediate()),
                    Opcode::EOR => rob_slot.result.push(rs.source[0].get_immediate() ^ rs.source[1].get_immediate()),
                    Opcode::MVN => rob_slot.result.push(!rs.source[0].get_immediate()),
                    Opcode::LDR => rob_slot.result.push(memory_subsystem.memory[rs.source[0].get_immediate() as usize]),
                    Opcode::STR => rob_slot.result.push(rs.source[0].get_immediate()),
                    Opcode::PRINTR => {
                        println!("PRINTR {}={}", Operand::Register(instr.source[0].get_register()), rs.source[0].get_immediate());
                    }
                    Opcode::CMP => {
                        let rn = rs.source[0].get_immediate();
                        let operand2 = rs.source[1].get_immediate();
                        let cprs_value = rs.source[2].get_immediate();

                        // Perform the comparison: rn - operand2
                        let result = rn.wrapping_sub(operand2);

                        // Update the CPSR flags based on the result
                        let zero_flag = result == 0;
                        let negative_flag = result < 0;
                        let carry_flag = (rn as u64).wrapping_sub(operand2 as u64) > (rn as u64); // Checking for borrow
                        let overflow_flag = ((rn ^ operand2) & (rn ^ result)) >> (std::mem::size_of::<i64>() * 8 - 1) != 0;

                        let mut new_cprs_value = cprs_value;
                        if zero_flag {
                            new_cprs_value |= 1 << ZERO_FLAG;
                        } else {
                            new_cprs_value &= !(1 << ZERO_FLAG);
                        }

                        if negative_flag {
                            new_cprs_value |= 1 << NEGATIVE_FLAG;
                        } else {
                            new_cprs_value &= !(1 << NEGATIVE_FLAG);
                        }

                        if carry_flag {
                            new_cprs_value |= 1 << CARRY_FLAG;
                        } else {
                            new_cprs_value &= !(1 << CARRY_FLAG);
                        }

                        if overflow_flag {
                            new_cprs_value |= 1 << OVERFLOW_FLAG;
                        } else {
                            new_cprs_value &= !(1 << OVERFLOW_FLAG);
                        }

                        // Update CPRS
                        rob_slot.result.push(new_cprs_value as i64);
                    }
                    Opcode::BEQ | Opcode::BNE | Opcode::BLT | Opcode::BLE | Opcode::BGT | Opcode::BGE => {
                        let target = rs.source[0].get_immediate();
                        let cpsr = rs.source[1].get_code_address();
                        let pc = rob_slot.pc as WordType;

                        let pc_update = match rs.opcode {
                            Opcode::BEQ => if cpsr == 0 { target } else { pc + 1 },
                            Opcode::BNE => if cpsr != 0 { target } else { pc + 1 },
                            Opcode::BLT => if cpsr < 0 { target } else { pc + 1 },
                            Opcode::BLE => if cpsr <= 0 { target } else { pc + 1 },
                            Opcode::BGT => if cpsr > 0 { target } else { pc + 1 },
                            Opcode::BGE => if cpsr >= 0 { target } else { pc + 1 },
                            _ => unreachable!("Unhandled opcode {:?}", rs.opcode),
                        };

                        rob_slot.branch_target_actual = pc_update as usize;
                    }
                    Opcode::CBZ | Opcode::CBNZ => {
                        let reg_value = rs.source[0].get_immediate();
                        let branch = rs.source[1].get_code_address();
                        let pc = rob_slot.pc as WordType;

                        let pc_update = match instr.opcode {
                            Opcode::CBZ => if reg_value == 0 { branch } else { pc + 1 },
                            Opcode::CBNZ => if reg_value != 0 { branch } else { pc + 1 },
                            _ => unreachable!("Unhandled opcode {:?}", rs.opcode),
                        };

                        rob_slot.branch_target_actual = pc_update as usize;
                    }
                    Opcode::B => {
                        // update the PC
                        let branch_target = rs.source[0].get_code_address();
                        let pc_update = branch_target;
                        rob_slot.branch_target_actual = pc_update as usize;
                    }
                    Opcode::BX => {
                        // update the PC
                        let branch_target = rs.source[0].get_immediate() as i64;
                        let pc_update = branch_target;
                        rob_slot.branch_target_actual = pc_update as usize;
                    }
                    Opcode::BL => {
                        let branch_target = rs.source[0].get_code_address();

                        let pc_update = branch_target;

                        // update LR
                        rob_slot.result.push((rob_slot.pc + 1) as WordType);
                        rob_slot.branch_target_actual = pc_update as usize;
                    }
                    Opcode::EXIT => {}
                    Opcode::DSB => {}
                }

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
                            memory_subsystem.sb.store(rob_slot.sb_pos.unwrap(), addr, result);
                        }
                        Operand::Immediate(_) | Operand::Code(_) | Operand::Unused => panic!("Illegal sink {:?}", sink),
                    }
                }

                let eu_index = eu.index;
                self.eu_table.deallocate(eu_index);
                rob_slot.eu_index = None;

                self.rs_table.deallocate(rs_index);
                rob_slot.rs_index = None;

                rob_slot.state = ROBSlotState::EXECUTED;
                perf_monitors.execute_cnt += 1;
            }
        }

        self.cdb_broadcast();
    }

    fn cdb_broadcast(&mut self) {
        let rs_table_capacity = self.rs_table.capacity;

        for req in &mut *self.cdb_broadcast_buffer {
            // Iterate over all RS and replace every matching physical register, by the value
            for rs_index in 0..rs_table_capacity {
                let rs = self.rs_table.get_mut(rs_index);
                if rs.state == RSState::IDLE {
                    continue;
                }

                let rob_slot_index = rs.rob_slot_index.unwrap();
                let rob_slot = self.rob.get_mut(rob_slot_index);
                if rob_slot.state != ROBSlotState::ISSUED {
                    continue;
                }

                let rs = self.rs_table.get_mut(rob_slot.rs_index.unwrap());
                let mut added_src_ready = false;
                for source_index in 0..rs.source_cnt as usize {
                    let source_rs = &mut rs.source[source_index];
                    if let Operand::Register(phys_reg) = source_rs {
                        if *phys_reg == req.phys_reg {
                            rs.source[source_index] = Operand::Immediate(req.value);
                            rs.source_ready_cnt += 1;
                            added_src_ready = true;
                        }
                    }
                }

                // bug: it can happen that the same rs is offered multiple times
                // one time it triggered when the allocation of rs is done
                // and the other time here.
                if added_src_ready && rs.source_cnt == rs.source_ready_cnt {
                    self.rs_table.enqueue_ready(rob_slot.rs_index.unwrap());
                }
            }
        }

        self.cdb_broadcast_buffer.clear();
    }

    fn cycle_retire(&mut self) {
        let mut bad_speculation = false;

        {
            let mut arch_reg_file = self.arch_reg_file.borrow_mut();
            let mut perf_monitors = self.perf_counters.borrow_mut();
            let phys_reg_file = &mut self.phys_reg_file;
            //let frontend_control = self.frontend_control.borrow_mut();
            let mut memory_subsytem = self.memory_subsystem.borrow_mut();

            for _ in 0..self.retire_n_wide {
                let rob_slot_index = self.rob.to_index(self.rob.seq_retired);
                let rob_slot = self.rob.get_mut(rob_slot_index);

                if rob_slot.state != ROBSlotState::EXECUTED {
                    break;
                }

                let rc = <Option<Rc<Instr>> as Clone>::clone(&rob_slot.instr).unwrap();
                let instr = Rc::clone(&rc);

                perf_monitors.retired_cnt += 1;

                if instr.opcode == Opcode::EXIT {
                    self.exit = true;
                }

                if self.trace.retire {
                    println!("Retiring {}", instr);
                }

                for sink_index in 0..instr.sink_cnt as usize {
                    let sink = instr.sink[sink_index];
                    match sink {
                        Operand::Register(arch_reg) => {
                            let rat_entry = self.rat.get_mut(arch_reg);
                            debug_assert!(rat_entry.valid);

                            let rat_phys_reg = rat_entry.phys_reg;
                            let rs_phys_reg = rob_slot.sink[sink_index].get_register();

                            // only when the physical register on the rat is the same as the physical register used for that
                            // instruction, the rat entry should be invalidated
                            if rat_phys_reg == rs_phys_reg {
                                rat_entry.valid = false;
                            }

                            phys_reg_file.get_mut(rs_phys_reg).has_value = false;
                            phys_reg_file.deallocate(rs_phys_reg);

                            arch_reg_file.set_value(arch_reg, rob_slot.result[sink_index]);
                        }
                        Operand::Memory(_) => {
                            memory_subsytem.sb.commit(rob_slot.sb_pos.unwrap())
                        }
                        _ => unreachable!(),
                    }
                }

                if instr.is_branch() {
                    if rob_slot.branch_target_actual != rob_slot.branch_target_predicted {
                        //println!("Branch prediction bad: actual={} predicted={}", rob_slot.branch_target_actual, rob_slot.branch_target_predicted);

                        // the branch was not correctly predicted
                        perf_monitors.branch_misprediction_cnt += 1;
                        bad_speculation = true;

                        // re-steer the frontend
                        arch_reg_file.set_value(PC, rob_slot.branch_target_actual as WordType);
                    } else {
                        //println!("Branch prediction good: actual={} predicted={}", rob_slot.branch_target_actual, rob_slot.branch_target_predicted);

                        // the branch was correctly predicted
                        perf_monitors.branch_good_predictions_cnt += 1;
                    }
                }

                self.rob.seq_retired += 1;
                self.rob.deallocate();

                if bad_speculation {
                    break;
                }
            }
        }

        if bad_speculation {
            self.flush();
        }
    }

    fn flush(&mut self) {
        let mut perf_monitors = self.perf_counters.borrow_mut();

        if self.trace.pipeline_flush {
            println!("Pipeline flush");
        }

        perf_monitors.pipeline_flushes += 1;
        perf_monitors.bad_speculation_cnt += self.rob.size() as u64;

        self.instr_queue.borrow_mut().flush();

        self.phys_reg_file.flush();
        self.eu_table.flush();
        self.rob.flush();
        self.rat.flush();
        self.rs_table.flush();
        self.memory_subsystem.borrow_mut().sb.flush();
        self.phys_reg_file.flush();
    }
}