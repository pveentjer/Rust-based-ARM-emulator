use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::physical_register::PhysRegFile;
use crate::backend::reorder_buffer::ROBSlot;
use crate::backend::reservation_station::{RS, RSDataProcessing, RSInstr, RSLoadStore, RSPrintr};
use crate::cpu::{CARRY_FLAG, CPUConfig, NEGATIVE_FLAG, OVERFLOW_FLAG, PerfCounters, ZERO_FLAG};
use crate::instructions::instructions::{DWordType, Opcode, Operand};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;

/// A single execution unit.
pub(crate) struct EU {
    pub(crate) index: u8,
    pub(crate) rs_index: Option<u16>,
    pub(crate) cycles_remaining: u8,
    pub(crate) state: EUState,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    perf_counters: Rc<RefCell<PerfCounters>>,
    phys_reg_file: Rc<RefCell<PhysRegFile>>,
    trace: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum EUState {
    IDLE,
    EXECUTING,
    COMPLETED,
}

#[allow(non_snake_case)]
impl EU {
    fn reset(&mut self) {
        self.rs_index = None;
        self.cycles_remaining = 0;
        self.state = EUState::IDLE;
    }

    pub fn cycle(&mut self,
                 rs: &mut RS,
                 rob_slot: &mut ROBSlot) {
        debug_assert!(self.state == EUState::EXECUTING);
        debug_assert!(self.cycles_remaining > 0);

        self.cycles_remaining -= 1;
        if self.cycles_remaining > 0 {
            // the execution unit isn't finished with its work
            return;
        }
        self.state = EUState::COMPLETED;
        self.perf_counters.borrow_mut().execute_cnt += 1;

        if self.trace {
            let instr = rob_slot.instr.as_ref().unwrap();
            println!("Executing {}", instr);
        }

        match &mut rs.instr {
            RSInstr::DataProcessing { data_processing } => self.execute_data_processing(data_processing, rob_slot),
            RSInstr::Branch { branch } => panic!(),
            RSInstr::LoadStore { load_store } => self.execute_load_store(load_store, rob_slot),
            RSInstr::Printr { printr } => self.execute_printr(printr),
            RSInstr::Synchronization { .. } => {}
        }
    }

    fn execute_printr(&mut self, printr: &mut RSPrintr) {
        println!("PRINTR {}={}", Operand::Register(printr.rn.arch_reg), printr.rn.value.unwrap())
    }

    fn execute_data_processing(&mut self, data_processing: &mut RSDataProcessing, rob_slot: &mut ROBSlot) {
        let result = match &data_processing.opcode {
            Opcode::ADD => self.execute_add(data_processing),
            Opcode::SUB => self.execute_sub(data_processing),
            Opcode::RSB => self.execute_rsb(data_processing),
            Opcode::MUL => self.execute_mul(data_processing),
            Opcode::MOV => self.execute_mov(data_processing),
            Opcode::CMP => self.execute_cmp(data_processing),
            Opcode::SDIV => self.execute_sdiv(data_processing),
            Opcode::AND => self.execute_and(data_processing),
            Opcode::ORR => self.execute_orr(data_processing),
            Opcode::EOR => self.execute_eor(data_processing),
            _ => unreachable!()
        };
        data_processing.rd.value = Some(result);
        self.phys_reg_file.borrow_mut().set_value(data_processing.rd.phys_reg.unwrap(), result);

        rob_slot.renamed_registers.push(data_processing.rd.clone())
    }

    fn execute_cmp(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();

        let rd_value = data_processing.rd_src.as_ref().unwrap().value.unwrap();

        // Perform the comparison: rn - operand2
        let result = rn_value.wrapping_sub(operand2_value);

        // Update the CPSR flags based on the result
        let zero_flag = result == 0;
        let negative_flag = (result & (1 << 63)) != 0;
        let carry_flag = (rn_value as u128).wrapping_sub(operand2_value as u128) > (rn_value as u128); // Checking for borrow
        let overflow_flag = (((rn_value ^ operand2_value) & (rn_value ^ result)) >> 63) != 0;

        let mut rd_update = rd_value;
        if zero_flag {
            rd_update |= 1 << ZERO_FLAG;
        } else {
            rd_update &= !(1 << ZERO_FLAG);
        }

        if negative_flag {
            rd_update |= 1 << NEGATIVE_FLAG;
        } else {
            rd_update &= !(1 << NEGATIVE_FLAG);
        }

        if carry_flag {
            rd_update |= 1 << CARRY_FLAG;
        } else {
            rd_update &= !(1 << CARRY_FLAG);
        }

        if overflow_flag {
            rd_update |= 1 << OVERFLOW_FLAG;
        } else {
            rd_update &= !(1 << OVERFLOW_FLAG);
        }

        rd_update
    }

    fn execute_sdiv(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();

        rn_value / operand2_value
    }

    fn execute_mov(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        data_processing.operand2.value()
    }

    fn execute_mul(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value * operand2_value
    }

    fn execute_rsb(&mut self, data_processing: &mut RSDataProcessing) -> u64 {    // let rn = rs.source[0].value.unwrap();
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        operand2_value.wrapping_sub(rn_value)
    }

    fn execute_sub(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value - operand2_value
    }

    fn execute_add(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value + operand2_value
    }

    fn execute_and(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value & operand2_value
    }

    fn execute_orr(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value | operand2_value
    }

    fn execute_eor(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value ^ operand2_value
    }

    fn execute_load_store(&mut self, load_store: &mut RSLoadStore, rob_slot: &mut ROBSlot) {
        match &load_store.opcode {
            Opcode::LDR => {
                let memory_subsystem = self.memory_subsystem.borrow_mut();
                let address = load_store.rn.value.unwrap() as usize;
                let value = memory_subsystem.memory[address];

                let rd = load_store.rd.phys_reg.unwrap();
                load_store.rd.value = Some(value);
                self.phys_reg_file.borrow_mut().set_value(rd, value);

                rob_slot.renamed_registers.push(load_store.rd.clone())
            }
            Opcode::STR => {
                let value = load_store.rd.value.unwrap();
                let address = load_store.rn.value.unwrap();

                let mut memory_subsystem = self.memory_subsystem.borrow_mut();
                memory_subsystem.sb.store(rob_slot.sb_pos.unwrap(), address, value);
            }
            _ => unreachable!()
        };
    }

    fn execute_BEQ(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let target = rs.source[0].value.unwrap();
        // let cpsr = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        // let pc_update = if zero_flag == 1 { target } else { pc + 1 };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BNE(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let target = rs.source[0].value.unwrap();
        // let cpsr = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        // let pc_update = if zero_flag == 0 { target } else { pc + 1 };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLT(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let target = rs.source[0].value.unwrap();
        // let cpsr = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        // let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;
        //
        // let pc_update = if negative_flag != overflow_flag { target } else { pc + 1 };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLE(&mut self, rs: &RS, rob_slot: &mut ROBSlot) {
        // let target = rs.source[0].value.unwrap();
        // let cpsr = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // // Extract the zero flag (bit 30), negative flag (bit 31), and overflow flag (bit 28) from CPSR
        // let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        // let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        // let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;
        //
        // let pc_update = if (zero_flag == 1) || (negative_flag != overflow_flag) {
        //     target
        // } else {
        //     pc + 1
        // };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGT(&mut self, rs: &RS, rob_slot: &mut ROBSlot) {
        // let target = rs.source[0].value.unwrap();
        // let cpsr = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        // let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        // let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;
        //
        // let pc_update = if (zero_flag == 0) && (negative_flag == overflow_flag) {
        //     target
        // } else {
        //     pc + 1
        // };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGE(&mut self, rs: &RS, rob_slot: &mut ROBSlot) {
        // let target = rs.source[0].value.unwrap();
        // let cpsr = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        // let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;
        //
        // let pc_update = if negative_flag == overflow_flag {
        //     target
        // } else {
        //     pc + 1
        // };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CBZ(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let reg_value = rs.source[0].value.unwrap();
        // let branch = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // let pc_update = if reg_value == 0 { branch } else { pc + 1 };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CBNZ(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let reg_value = rs.source[0].value.unwrap();
        // let branch = rs.source[1].value.unwrap();
        // let pc = rob_slot.pc as DWordType;
        //
        // let pc_update = if reg_value != 0 { branch } else { pc + 1 };
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BL(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let branch_target = rs.source[0].value.unwrap();
        //
        // let pc_update = branch_target;
        //
        // // update LR
        // let value = (rob_slot.pc + 1) as DWordType;
        // let dst_phys_reg = rs.sink[0].phys_reg.unwrap();
        // self.phys_reg_file.borrow_mut().set_value(dst_phys_reg, value);
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BX(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // // update the PC
        // let branch_target = rs.source[0].value.unwrap() as i64;
        // let pc_update = branch_target;
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_B(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // // update the PC
        // let branch_target = rs.source[0].value.unwrap();
        // let pc_update = branch_target;
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_RET(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // // update the PC
        // let branch_target = rs.source[0].value.unwrap();
        // let pc_update = branch_target;
        // rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_PRINTR(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let instr = rob_slot.instr.as_ref().unwrap();
        //
        // println!("PRINTR {}={}", Operand::Register(instr.source[0].get_register()), rs.source[0].value.unwrap());
    }

    fn execute_STR(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // let value = rs.source[0].value.unwrap();
        // let address = rs.source[1].value.unwrap();
        //
        // let mut memory_subsystem = self.memory_subsystem.borrow_mut();
        // memory_subsystem.sb.store(rob_slot.sb_pos.unwrap(), address, value);
    }

    fn execute_LDR(&mut self, rs: &mut RS) {
        // let memory_subsystem = self.memory_subsystem.borrow_mut();
        // let address = rs.source[0].value.unwrap() as usize;
        // let value = memory_subsystem.memory[address];
        // let dst_phys_reg = rs.sink[0].phys_reg.unwrap();
        // self.phys_reg_file.borrow_mut().set_value(dst_phys_reg, value);
    }

    fn execute_MVN(&mut self, rs: &mut RS) {
        // let value = !rs.source[0].value.unwrap();
        // let dst_phys_reg = rs.sink[0].phys_reg.unwrap();
        // self.phys_reg_file.borrow_mut().set_value(dst_phys_reg, value);
    }

    fn execute_ADR(&mut self, _rs: &mut RS, _rob_slot: &mut ROBSlot) {
        panic!("ADR is not implemented");
    }

    fn execute_NEG(&mut self, rs: &mut RS) {
        // let rn = rs.source[0].value.unwrap();
        // let rd = rn.wrapping_neg();
        // let dst_phys_reg = rs.sink[0].phys_reg.unwrap();
        // self.phys_reg_file.borrow_mut().set_value(dst_phys_reg, rd);
    }

    fn execute_SDIV(&mut self, rs: &mut RS) {
        // let rn = rs.source[0].value.unwrap();
        // let operand2 = rs.source[1].value.unwrap();
        // let rd = rn / operand2;
        // let dst_phys_reg = rs.sink[0].phys_reg.unwrap();
        // self.phys_reg_file.borrow_mut().set_value(dst_phys_reg, rd);
    }

}

/// The table containing all execution units of a CPU core.
pub(crate) struct EUTable {
    pub(crate) capacity: u8,
    idle_stack: Vec<u8>,
    array: Vec<EU>,
}

impl EUTable {
    pub(crate) fn new(
        cpu_config: &CPUConfig,
        memory_subsystem: &Rc<RefCell<MemorySubsystem>>,
        phys_reg_file: &Rc<RefCell<PhysRegFile>>,
        perf_counters: &Rc<RefCell<PerfCounters>>,
    ) -> EUTable {
        let capacity = cpu_config.eu_count;
        let mut free_stack = Vec::with_capacity(capacity as usize);
        let mut array = Vec::with_capacity(capacity as usize);
        for i in 0..capacity {
            array.push(EU {
                index: i,
                cycles_remaining: 0,
                rs_index: None,
                state: EUState::IDLE,
                trace: cpu_config.trace.execute,
                memory_subsystem: Rc::clone(memory_subsystem),
                perf_counters: Rc::clone(perf_counters),
                phys_reg_file: Rc::clone(phys_reg_file),
            });
            free_stack.push(i);
        }

        EUTable {
            capacity,
            array,
            idle_stack: free_stack,
        }
    }

    pub(crate) fn flush(&mut self) {
        self.idle_stack.clear();
        for k in 0..self.capacity {
            self.idle_stack.push(k);
            self.array.get_mut(k as usize).unwrap().reset();
        }
    }

    pub(crate) fn has_idle(&self) -> bool {
        return !self.idle_stack.is_empty();
    }

    pub(crate) fn get_mut(&mut self, eu_index: u8) -> &mut EU {
        self.array.get_mut(eu_index as usize).unwrap()
    }

    pub(crate) fn allocate(&mut self) -> u8 {
        if let Some(last_element) = self.idle_stack.pop() {
            let eu = self.array.get_mut(last_element as usize).unwrap();
            debug_assert!(eu.state == EUState::IDLE);
            debug_assert!(eu.rs_index.is_none());
            debug_assert!(eu.cycles_remaining == 0);

            eu.state = EUState::EXECUTING;
            return last_element;
        } else {
            panic!("No free PhysReg")
        }
    }

    pub(crate) fn deallocate(&mut self, eu_index: u8) {
        let eu = self.array.get_mut(eu_index as usize).unwrap();
        debug_assert!(eu.state == EUState::EXECUTING || eu.state == EUState::COMPLETED);
        debug_assert!(eu.rs_index.is_some());
        debug_assert!(!self.idle_stack.contains(&eu_index));

        eu.reset();
        self.idle_stack.push(eu_index);
    }
}
