use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::execution_unit::{EUState, EUTable};
use crate::backend::physical_register::PhysRegFile;
use crate::backend::register_alias_table::RAT;
use crate::backend::reorder_buffer::{ROB, ROBSlotState};
use crate::backend::reservation_station::{RSState, RSTable};
use crate::cpu::{ArgRegFile, CPUConfig, PC, PerfCounters, Trace};
use crate::frontend::frontend::FrontendControl;
use crate::instructions::instructions::{DWordType, InstrQueue, Opcode, Operand, RegisterType};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;

struct CDBBroadcast {
    phys_reg: RegisterType,
    value: DWordType,
}

pub(crate) struct Backend {
    instr_queue: Rc<RefCell<InstrQueue>>,
    arch_reg_file: Rc<RefCell<ArgRegFile>>,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    frontend_control: Rc<RefCell<FrontendControl>>,
    rs_table: RSTable,
    phys_reg_file: Rc<RefCell<PhysRegFile>>,
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

        let mut phys_reg_file = Rc::new(RefCell::new(PhysRegFile::new(cpu_config.phys_reg_count)));

        Backend {
            trace: cpu_config.trace.clone(),
            instr_queue,
            memory_subsystem:Rc::clone(&memory_subsystem),
            arch_reg_file,
            rs_table: RSTable::new(cpu_config.rs_count),
            phys_reg_file: Rc::clone(&phys_reg_file),
            rat: RAT::new(cpu_config.phys_reg_count),
            rob: ROB::new(cpu_config.rob_capacity),
            eu_table: EUTable::new(cpu_config, &memory_subsystem, &phys_reg_file, &perf_counters),
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
        let mut perf_counters = self.perf_counters.borrow_mut();
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
            perf_counters.issue_cnt += 1;

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

            let instr = rob_slot.instr.as_ref().unwrap();

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
            for operand_index in 0..instr.source_cnt as usize {
                let operand_instr = &instr.source[operand_index];
                let mut operand_rs = &mut rs.source[operand_index];
                operand_rs.operand = Some(*operand_instr);
                match operand_instr {
                    Operand::Register(arch_reg) => {
                        let rat_entry = self.rat.get(*arch_reg);
                        if rat_entry.valid {
                            let phys_reg_file = self.phys_reg_file.borrow_mut();
                            let phys_reg_entry = phys_reg_file.get(rat_entry.phys_reg);
                            if phys_reg_entry.has_value {
                                //we got lucky, there is a value in the physical register.
                                operand_rs.value = Some(phys_reg_entry.value);
                                rs.source_ready_cnt += 1;
                            } else {
                                // cdb broadcast will update
                                operand_rs.phys_reg = Some(rat_entry.phys_reg);
                            }
                        } else {
                            let value = arch_reg_file.get_value(*arch_reg);
                            operand_rs.value = Some(value);
                            rs.source_ready_cnt += 1;
                        }
                    }
                    Operand::Memory(addr)  => {
                        operand_rs.value=Some(*addr);
                        rs.source_ready_cnt += 1;
                    }
                    Operand::Code(addr) => {
                        operand_rs.value=Some(*addr);
                        rs.source_ready_cnt += 1;
                    }
                    Operand::Immediate(value) => {
                        operand_rs.value=Some(*value);
                        rs.source_ready_cnt += 1;
                    }
                    Operand::Unused =>
                        panic!("Illegal source {:?}", operand_instr)
                }
            }

            // Register renaming of the sink operands.
            rs.sink_cnt = instr.sink_cnt;
            for operand_index in 0..instr.sink_cnt as usize {
                let operand_instr = &instr.sink[operand_index];
                let mut operand_rs = &mut rs.sink[operand_index];
                operand_rs.operand = Some(*operand_instr);
                match operand_instr {
                    Operand::Register(arch_reg) => {
                        let phys_reg = self.phys_reg_file.borrow_mut().allocate();
                        println!("Allocated phys register {}",phys_reg);
                        // update the RAT entry to point to the newest phys_reg
                        let rat_entry = self.rat.get_mut(*arch_reg);
                        rat_entry.phys_reg = phys_reg;
                        rat_entry.valid = true;

                        rob_slot.sink_phys_regs[operand_index]=Some(phys_reg);

                        operand_rs.phys_reg = Some(phys_reg);
                    }
                    Operand::Memory(_) => {
                        println!("Backend allocating SB entry");
                        // since the instructions are issued in program order, a slot is allocated in the
                        // sb in program order. And since sb will commit to the coherent cache
                        // (in this case directly to memory), the stores will become visible
                        // in program order.
                        rob_slot.sb_pos = Some(memory_subsystem.sb.allocate());
                    }
                    Operand::Unused | Operand::Immediate(_) | Operand::Code(_) => {
                        panic!("Illegal sink {:?}", operand_instr)
                    }
                }
            }

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
        let mut perf_counters = self.perf_counters.borrow_mut();

        for _ in 0..self.dispatch_n_wide {
            if !self.rs_table.has_ready() || !self.eu_table.has_idle() {
                break;
            }

            let rs_index = self.rs_table.deque_ready();

            let rs = self.rs_table.get_mut(rs_index);
            debug_assert!(rs.state == RSState::BUSY);

            let rob_slot_index = rs.rob_slot_index.unwrap();
            let rob_slot = self.rob.get_mut(rob_slot_index);

            let eu_index = self.eu_table.allocate();
            let eu = self.eu_table.get_mut(eu_index);
            debug_assert!(eu.state == EUState::EXECUTING);

            let instr = rob_slot.instr.as_ref().unwrap();

            eu.rs_index = Some(rs_index);
            eu.cycles_remaining = instr.cycles;

            rob_slot.state = ROBSlotState::DISPATCHED;
            rob_slot.eu_index = Some(eu_index);

            if self.trace.dispatch {
                println!("Dispatched [{}]", instr);
            }
            perf_counters.dispatch_cnt += 1;
        }
    }

    fn cycle_eu_table(&mut self) {
        {
            // todo: we should only iterate over the used execution units.
            for eu_index in 0..self.eu_table.capacity {
                let eu = self.eu_table.get_mut(eu_index);

                if eu.state == EUState::IDLE {
                    continue;
                }

                debug_assert!(eu.state == EUState::EXECUTING);

                let rs_index = eu.rs_index.unwrap();

                let rs = self.rs_table.get_mut(rs_index);
                debug_assert!(rs.state == RSState::BUSY);

                let rob_index = rs.rob_slot_index.unwrap();
                let rob_slot = self.rob.get_mut(rob_index);
                debug_assert!(rob_slot.state == ROBSlotState::DISPATCHED,
                              "rob_slot is not in dispatched state, but in {:?}, rs_index={}", rob_slot.state, rs_index);
                debug_assert!(rob_slot.rs_index.is_some());
                debug_assert!(rob_slot.eu_index.is_some());

                eu.cycle(rs, rob_slot);

                if eu.state == EUState::EXECUTING {
                    continue;
                }

                debug_assert!(eu.state == EUState::COMPLETED);

                for sink_index in 0..rs.sink_cnt {
                    let sink = &mut rs.sink[sink_index as usize];
                    match sink.operand.unwrap() {
                        Operand::Register(_) => {
                            let phys_reg = sink.phys_reg.unwrap();
                            let mut phys_reg_file = self.phys_reg_file.borrow_mut();
                            let phys_reg_entry = phys_reg_file.get_mut(phys_reg);
                            phys_reg_entry.has_value = true;
                            let result = rob_slot.result[sink_index as usize];
                            phys_reg_entry.value = result;
                            self.cdb_broadcast_buffer.push(CDBBroadcast { phys_reg, value: result });
                        }
                        Operand::Memory(addr) => {
                            let result = rob_slot.result[sink_index as usize];
                            // a store to memory
                            let mut memory_subsystem = self.memory_subsystem.borrow_mut();
                            memory_subsystem.sb.store(rob_slot.sb_pos.unwrap(), addr, result);
                        }
                        Operand::Immediate(_) |
                        Operand::Code(_) |
                        Operand::Unused => panic!("Illegal sink {:?}", sink.operand.unwrap()),
                    }
                }

                let eu_index = eu.index;
                self.eu_table.deallocate(eu_index);
                rob_slot.eu_index = None;

                self.rs_table.deallocate(rs_index);
                rob_slot.rs_index = None;

                rob_slot.state = ROBSlotState::EXECUTED;
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
                    let operand_rs = &mut rs.source[source_index];
                    if let Some(phys_reg) = operand_rs.phys_reg {
                        if phys_reg == req.phys_reg {
                            operand_rs.value = Some(req.value);
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
            let mut perf_counters = self.perf_counters.borrow_mut();
            let mut phys_reg_file = &mut self.phys_reg_file.borrow_mut();
            //let frontend_control = self.frontend_control.borrow_mut();
            let mut memory_subsytem = self.memory_subsystem.borrow_mut();

            for _ in 0..self.retire_n_wide {
                let rob_slot_index = self.rob.to_index(self.rob.seq_retired);
                let rob_slot = self.rob.get_mut(rob_slot_index);

                if rob_slot.state != ROBSlotState::EXECUTED {
                    break;
                }

                let instr = rob_slot.instr.as_ref().unwrap();

                perf_counters.retired_cnt += 1;

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
                            let rs_phys_reg = rob_slot.sink_phys_regs[sink_index].unwrap();

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
                        perf_counters.branch_miss_prediction_cnt += 1;
                        bad_speculation = true;

                        // re-steer the frontend
                        arch_reg_file.set_value(PC, rob_slot.branch_target_actual as DWordType);
                    } else {
                        //println!("Branch prediction good: actual={} predicted={}", rob_slot.branch_target_actual, rob_slot.branch_target_predicted);

                        // the branch was correctly predicted
                        perf_counters.branch_good_predictions_cnt += 1;
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
        let mut perf_counters = self.perf_counters.borrow_mut();

        if self.trace.pipeline_flush {
            println!("Pipeline flush");
        }

        perf_counters.pipeline_flushes += 1;
        perf_counters.bad_speculation_cnt += self.rob.size() as u64;

        self.instr_queue.borrow_mut().flush();
        self.phys_reg_file.borrow_mut().flush();
        self.eu_table.flush();
        self.rob.flush();
        self.rat.flush();
        self.rs_table.flush();
        self.memory_subsystem.borrow_mut().sb.flush();
    }
}