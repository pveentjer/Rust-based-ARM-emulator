use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::execution_unit::{EUState, EUTable};
use crate::backend::physical_register::PhysRegFile;
use crate::backend::register_alias_table::RAT;
use crate::backend::reorder_buffer::{ROB, ROBSlotState};
use crate::backend::reservation_station::{RenamedRegister, RS, RSDataProcessing, RSInstr, RSLoadStore, RSPrintr, RSState, RSTable};
use crate::cpu::{ArgRegFile, CPUConfig, PerfCounters, Trace};
use crate::frontend::frontend::FrontendControl;
use crate::instructions::instructions::{DWordType, Instr, InstrQueue, Opcode, RegisterType};
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
    rob: Rc<RefCell<ROB>>,
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
    pub(crate) fn new(
        cpu_config: &CPUConfig,
        instr_queue: &Rc<RefCell<InstrQueue>>,
        memory_subsystem: &Rc<RefCell<MemorySubsystem>>,
        arch_reg_file: &Rc<RefCell<ArgRegFile>>,
        frontend_control: &Rc<RefCell<FrontendControl>>,
        perf_counters: &Rc<RefCell<PerfCounters>>,
    ) -> Backend {
        let phys_reg_file = Rc::new(RefCell::new(PhysRegFile::new(cpu_config.phys_reg_count)));

        Backend {
            trace: cpu_config.trace.clone(),
            instr_queue: Rc::clone(instr_queue),
            memory_subsystem: Rc::clone(&memory_subsystem),
            arch_reg_file: Rc::clone(arch_reg_file),
            rs_table: RSTable::new(cpu_config.rs_count),
            phys_reg_file: Rc::clone(&phys_reg_file),
            rat: RAT::new(cpu_config.phys_reg_count),
            rob: Rc::new(RefCell::new(ROB::new(cpu_config.rob_capacity))),
            eu_table: EUTable::new(cpu_config, &memory_subsystem, &phys_reg_file, &perf_counters),
            retire_n_wide: cpu_config.retire_n_wide,
            dispatch_n_wide: cpu_config.dispatch_n_wide,
            issue_n_wide: cpu_config.issue_n_wide,
            cdb_broadcast_buffer: Vec::with_capacity(cpu_config.eu_count as usize),
            frontend_control: Rc::clone(frontend_control),
            exit: false,
            perf_counters: Rc::clone(perf_counters),
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
        let mut rob = self.rob.borrow_mut();

        // try to put as many instructions into the rob
        for _ in 0..self.issue_n_wide {
            // println!("cycle_issue: instr_queue.isempty: {}, self.rob.has_space: {}", instr_queue.is_empty(), self.rob.has_space());

            if instr_queue.is_empty() || !rob.has_space() {
                break;
            }

            // todo: register renaming should be done here.

            let instr_queue_head_index = instr_queue.head_index();
            let instr_queue_slot = instr_queue.get_mut(instr_queue_head_index);

            let branch_target_predicted = instr_queue_slot.branch_target_predicted;
            let instr = Rc::clone(&instr_queue_slot.instr);

            // // If needed, synchronize of the sb being empty
            // if instr.sb_sync() && self.memory_subsystem.borrow().sb.size() > 0 {
            //     return;
            // }
            //
            // // If needed, synchronize on the rob being empty
            // if instr.rob_sync() && self.rob.size() > 0 {
            //     return;
            // }

            let rob_slot_index = rob.allocate();
            let rob_slot = rob.get_mut(rob_slot_index);

            if self.trace.issue {
                println!("Issued [{}]", instr);
            }

            rob_slot.pc = instr_queue_slot.pc;
            rob_slot.state = ROBSlotState::ISSUED;
            rob_slot.instr = Some(instr);
            rob_slot.branch_target_predicted = branch_target_predicted;
            rob.seq_issued += 1;
            perf_counters.issue_cnt += 1;

            instr_queue.head_bump();
        }
    }

    // For any rob entry that doesn't have a reservation station, try to look up a rs.
    fn cycle_rs_allocation(&mut self) {
        let arch_reg_file = self.arch_reg_file.borrow();
        let mut phys_reg_file = self.phys_reg_file.borrow_mut();
        let mut memory_subsystem = self.memory_subsystem.borrow_mut();
        let mut rob = self.rob.borrow_mut();

        for _ in 0..self.issue_n_wide {
            if rob.seq_rs_allocated == rob.seq_issued || !self.rs_table.has_idle() {
                break;
            }

            let rob_slot_index = rob.to_index(rob.seq_rs_allocated);
            let rob_slot = rob.get_mut(rob_slot_index);

            debug_assert!(rob_slot.state == ROBSlotState::ISSUED);
            debug_assert!(rob_slot.eu_index.is_none());
            debug_assert!(rob_slot.rs_index.is_none());

            let instr = rob_slot.instr.as_ref().unwrap();
            //
            // if instr.mem_stores > 0 {
            //     if !memory_subsystem.sb.has_space() {
            //         // we can't allocate a slot in the store buffer, we are done
            //         break;
            //     }
            //
            //     rob_slot.sb_pos = Some(memory_subsystem.sb.allocate());
            // }

            let rs_index = self.rs_table.allocate();
            let rs = self.rs_table.get_mut(rs_index);
            debug_assert!(rs.state == RSState::BUSY);

            rob_slot.rs_index = Some(rs_index);

            rs.rob_slot_index = Some(rob_slot_index);
            //todo
            //rs.opcode = instr.opcode();
            //rs.source_cnt = instr.source_cnt;

            match instr.as_ref() {
                Instr::DataProcessing { data_processing: data_processing } => {
                    rs.instr = RSInstr::DataProcessing {
                        fields: RSDataProcessing {
                            opcode: data_processing.opcode,
                            condition: data_processing.condition,
                            rn: register_rename_src(
                                data_processing.rn,
                                rs,
                                &mut self.rat,
                                &arch_reg_file,
                                &mut phys_reg_file),
                            rd: register_rename_sink(
                                data_processing.rd,
                                &mut phys_reg_file,
                                &mut self.rat),
                            operand2: 0,
                        }
                    };

                    println!("dataprocessing rs.pending_cnt: {}", rs.pending_cnt)
                }
                Instr::Branch { branch} => {}
                Instr::LoadStore {load_store} => {
                    match load_store.opcode {
                        Opcode::LDR => {
                            rs.instr = RSInstr::LoadStore {
                                fields: RSLoadStore {
                                    opcode: load_store.opcode,
                                    condition: load_store.condition,
                                    rn: register_rename_src(
                                        load_store.rn,
                                        rs,
                                        &mut self.rat,
                                        &arch_reg_file,
                                        &mut phys_reg_file),
                                    rt: register_rename_sink(
                                        load_store.rt,
                                        &mut phys_reg_file,
                                        &mut self.rat),
                                    offset: load_store.offset,
                                }
                            };
                        }
                        Opcode::STR => {
                            rs.instr = RSInstr::LoadStore {
                                fields: RSLoadStore {
                                    opcode: load_store.opcode,
                                    condition: load_store.condition,
                                    rn: register_rename_src(
                                        load_store.rn,
                                        rs,
                                        &mut self.rat,
                                        &arch_reg_file,
                                        &mut phys_reg_file),
                                    rt: register_rename_src(
                                        load_store.rt,
                                        rs,
                                        &mut self.rat,
                                        &arch_reg_file,
                                        &mut phys_reg_file),
                                    offset: load_store.offset,
                                }
                            };
                        }
                        _ => unreachable!(),
                    }
                }
                Instr::Printr { printr} => {
                    // rob_slot.sink_phys_regs[operand_index] = Some(phys_reg);
                    //
                    // operand_rs.phys_reg = Some(phys_reg);

                    rs.instr = RSInstr::Printr {
                        fields: RSPrintr {
                            rn: register_rename_src(
                                printr.rn,
                                rs,
                                &mut self.rat,
                                &arch_reg_file,
                                &mut phys_reg_file)
                        },
                    };

                    println!("dataprocessing rs.pending_cnt: {}", rs.pending_cnt)
                }
                Instr::Synchronization { .. } => {}
            }

            if rs.pending_cnt == 0 {
                println!("foobar");
                self.rs_table.enqueue_ready(rs_index);
            }

            if self.trace.allocate_rs {
                println!("Allocate RS [{}]", instr);
            }

            rob.seq_rs_allocated += 1;
        }
    }

    fn cycle_dispatch(&mut self) {
        let mut perf_counters = self.perf_counters.borrow_mut();
        let mut rob = self.rob.borrow_mut();

        for _ in 0..self.dispatch_n_wide {
            if !self.rs_table.has_ready() || !self.eu_table.has_idle() {
                break;
            }

            let rs_index = self.rs_table.deque_ready();

            let rs = self.rs_table.get_mut(rs_index);
            debug_assert!(rs.state == RSState::BUSY);

            let rob_slot_index = rs.rob_slot_index.unwrap();
            let rob_slot = rob.get_mut(rob_slot_index);

            let eu_index = self.eu_table.allocate();
            let eu = self.eu_table.get_mut(eu_index);
            debug_assert!(eu.state == EUState::EXECUTING);

            let instr = rob_slot.instr.as_ref().unwrap();

            eu.rs_index = Some(rs_index);

            // todo: correctly configure the cycles
            eu.cycles_remaining = 1;

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
            let mut rob = self.rob.borrow_mut();
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
                let rob_slot = rob.get_mut(rob_index);
                debug_assert!(rob_slot.state == ROBSlotState::DISPATCHED,
                              "rob_slot is not in dispatched state, but in {:?}, rs_index={}", rob_slot.state, rs_index);
                debug_assert!(rob_slot.rs_index.is_some());
                debug_assert!(rob_slot.eu_index.is_some());

                eu.cycle(rs, rob_slot);

                if eu.state == EUState::EXECUTING {
                    continue;
                }

                debug_assert!(eu.state == EUState::COMPLETED);

                // match rs.foobar {
                //     RS_Instr::DataProcessing { opcode, condition, rn, rd,operand2 } => {
                //         let mut phys_reg_file = self.phys_reg_file.borrow_mut();
                //         let phys_reg_entry = phys_reg_file.get_mut(rd_p);
                //         self.cdb_broadcast_buffer.push(CDBBroadcast { phys_reg: rd_p, value: phys_reg_entry.value });
                //     }
                //     RS_Instr::Branch { .. } => {}
                //     RS_Instr::LoadStore { .. } => {}
                //     RS_Instr::Printr { .. } => {}
                //     RS_Instr::Nop => {}
                //     RS_Instr::Exit => {}
                // }

                //
                // for sink_index in 0..rs.sink_cnt {
                //     let sink = &mut rs.sink[sink_index as usize];
                //     match sink.operand.unwrap() {
                //         Operand::Register(_) => {
                //             let phys_reg = sink.phys_reg.unwrap();
                //             let mut phys_reg_file = self.phys_reg_file.borrow_mut();
                //             let phys_reg_entry = phys_reg_file.get_mut(phys_reg);
                //             self.cdb_broadcast_buffer.push(CDBBroadcast { phys_reg, value: phys_reg_entry.value });
                //         }
                //         Operand::Memory(addr) => {}
                //         Operand::Immediate(_) |
                //         Operand::Code(_) |
                //         Operand::MemRegisterIndirect(_) |
                //         Operand::Unused => panic!("Illegal sink {:?}", sink.operand.unwrap()),
                //     }
                // }

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
        let mut rob = self.rob.borrow_mut();

        for broadcast in &mut *self.cdb_broadcast_buffer {
            // Iterate over all RS and replace every matching physical register, by the value
            for rs_index in 0..rs_table_capacity {
                let rs = self.rs_table.get_mut(rs_index);
                if rs.state == RSState::IDLE {
                    continue;
                }

                let rob_slot_index = rs.rob_slot_index.unwrap();
                let rob_slot = rob.get_mut(rob_slot_index);
                if rob_slot.state != ROBSlotState::ISSUED {
                    continue;
                }

                let rs = self.rs_table.get_mut(rob_slot.rs_index.unwrap());
                let mut resolved = false;

                // match rs.foobar {
                //     RS_Instr::DataProcessing { opcode, condition, rn_a, rn_p, ref mut rn_v, rd_a, rd_p, operand2 } => {
                //         if let Some(r) = rn_p {
                //             if r == broadcast.phys_reg {
                //                 *rn_v = Some(broadcast.value);
                //                 resolved = true;
                //                 rs.pending_cnt -= 1;
                //             }
                //         }
                //     }
                //     RS_Instr::Branch { .. } => {}
                //     RS_Instr::LoadStore { .. } => {}
                //     RS_Instr::Printr { rn_a, rn_p, ref mut rn_v } => {
                //         if let Some(r) = rn_p {
                //             if r == broadcast.phys_reg {
                //                 *rn_v = Some(broadcast.value);
                //                 resolved = true;
                //                 rs.pending_cnt -= 1;
                //             }
                //         }
                //     }
                //     RS_Instr::Nop => {}
                //     RS_Instr::Exit => {}
                // }

                // for source_index in 0..rs.source_cnt as usize {
                //     let operand_rs = &mut rs.source[source_index];
                //     if let Some(phys_reg) = operand_rs.phys_reg {
                //         if phys_reg == broadcast.phys_reg {
                //             operand_rs.value = Some(broadcast.value);
                //             rs.source_ready_cnt += 1;
                //             added_src_ready = true;
                //         }
                //     }
                // }

                // bug: it can happen that the same rs is offered multiple times
                // one time it triggered when the allocation of rs is done
                // and the other time here.
                if resolved && rs.pending_cnt == 0 {
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
            let mut rob = self.rob.borrow_mut();

            for _ in 0..self.retire_n_wide {
                let rob_slot_index = rob.to_index(rob.seq_retired);
                let rob_slot = rob.get_mut(rob_slot_index);

                if rob_slot.state != ROBSlotState::EXECUTED {
                    break;
                }

                let instr = rob_slot.instr.as_ref().unwrap();

                perf_counters.retired_cnt += 1;

                if let Instr::Synchronization { synchronization} = instr.as_ref() {
                    if synchronization.opcode == Opcode::EXIT {
                        self.exit = true;
                    }
                }

                if self.trace.retire {
                    println!("Retiring {}", instr);
                }

                // for sink_index in 0..instr.sink_cnt as usize {
                //     let sink = instr.sink[sink_index];
                //     match sink {
                //         Operand::Register(arch_reg) => {
                //             let rat_entry = self.rat.get_mut(arch_reg);
                //             debug_assert!(rat_entry.valid);
                //
                //             let rat_phys_reg = rat_entry.phys_reg;
                //             let rob_phys_reg = rob_slot.sink_phys_regs[sink_index].unwrap();
                //
                //             // only when the physical register on the rat is the same as the physical register used for that
                //             // instruction, the rat entry should be invalidated
                //             if rat_phys_reg == rob_phys_reg {
                //                 rat_entry.valid = false;
                //             }
                //
                //             // update the architectural register
                //             let value = phys_reg_file.get_value(rob_phys_reg);
                //             arch_reg_file.set_value(sink.get_register(), value);
                //
                //             phys_reg_file.deallocate(rob_phys_reg);
                //         }
                //         _ => unreachable!(),
                //     }
                // }

                if rob_slot.sb_pos.is_some() {
                    memory_subsytem.sb.commit(rob_slot.sb_pos.unwrap())
                }

                // if instr.is_branch() {
                //     if rob_slot.branch_target_actual != rob_slot.branch_target_predicted {
                //         // the branch was not correctly predicted
                //         perf_counters.branch_miss_prediction_cnt += 1;
                //         bad_speculation = true;
                //
                //         // re-steer the frontend
                //         arch_reg_file.set_value(PC, rob_slot.branch_target_actual as DWordType);
                //     } else {
                //         // the branch was correctly predicted
                //         perf_counters.branch_good_predictions_cnt += 1;
                //     }
                // }

                rob.seq_retired += 1;
                rob.deallocate();

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
        let mut rob = self.rob.borrow_mut();

        if self.trace.pipeline_flush {
            println!("Pipeline flush");
        }

        perf_counters.pipeline_flushes += 1;
        perf_counters.bad_speculation_cnt += rob.size() as u64;

        self.instr_queue.borrow_mut().flush();
        self.phys_reg_file.borrow_mut().flush();
        self.eu_table.flush();
        rob.flush();
        self.rat.flush();
        self.rs_table.flush();
        self.memory_subsystem.borrow_mut().sb.flush();
    }
}

fn register_rename_src(arch_reg: RegisterType,
                       rs: &mut RS,
                       rat: &mut RAT,
                       arch_reg_file: &ArgRegFile,
                       phys_reg_file: &mut PhysRegFile,
) -> RenamedRegister {
    let mut phys_reg = None;
    let mut value = None;
    let rat_entry = rat.get(arch_reg);
    if rat_entry.valid {
        let phys_reg_entry = phys_reg_file.get(rat_entry.phys_reg);
        if phys_reg_entry.has_value {
            //we got lucky, there is a value in the physical register.
            value = Some(phys_reg_entry.value);
        } else {
            rs.pending_cnt += 1;
            // cdb broadcast will update
            phys_reg = Some(rat_entry.phys_reg);
        }
    } else {
        println!("Reading physical register");
        value = Some(arch_reg_file.get_value(arch_reg));
    }

    RenamedRegister { arch_reg, phys_reg, value }
}

fn register_rename_sink(arch_reg: RegisterType,
                        phys_reg_file: &mut PhysRegFile,
                        rat: &mut RAT,
) -> RenamedRegister {
    let phys_reg = phys_reg_file.allocate();
    rat.update(arch_reg, phys_reg);

    RenamedRegister { arch_reg, phys_reg: Some(phys_reg), value: None }
}