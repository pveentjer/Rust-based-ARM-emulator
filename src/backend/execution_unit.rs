use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::physical_register::PhysRegFile;
use crate::backend::reorder_buffer::ROBSlot;
use crate::backend::reservation_station::{RS, RSBranch, RSDataProcessing, RSInstr, RSLoadStore, RSPrintr};
use crate::cpu::{CARRY_FLAG, CPUConfig, NEGATIVE_FLAG, OVERFLOW_FLAG, PerfCounters, ZERO_FLAG};
use crate::instructions::instructions::{DWordType, Opcode, RegisterTypeDisplay};
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
            RSInstr::Branch { branch } => self.execute_branch(branch, rob_slot),
            RSInstr::LoadStore { load_store } => self.execute_load_store(load_store, rob_slot),
            RSInstr::Printr { printr } => self.execute_printr(printr),
            RSInstr::Synchronization { .. } => {}
        }
    }

    fn execute_printr(&mut self, printr: &mut RSPrintr) {
        println!("PRINTR {}={}", RegisterTypeDisplay { register: printr.rn.arch_reg }, printr.rn.value.unwrap())
    }

    fn execute_data_processing(&mut self, data_processing: &mut RSDataProcessing, rob_slot: &mut ROBSlot) {

        // todo: here the conditional execution can be added

        let result = match &data_processing.opcode {
            Opcode::ADD => self.execute_ADD(data_processing),
            Opcode::SUB => self.execute_SUB(data_processing),
            Opcode::RSB => self.execute_RSB(data_processing),
            Opcode::MUL => self.execute_MUL(data_processing),
            Opcode::MOV => self.execute_MOV(data_processing),
            Opcode::CMP => self.execute_CMP(data_processing),
            Opcode::SDIV => self.execute_SDIV(data_processing),
            Opcode::AND => self.execute_AND(data_processing),
            Opcode::ORR => self.execute_ORR(data_processing),
            Opcode::EOR => self.execute_EOR(data_processing),
            Opcode::NEG => self.execute_NEG(data_processing),
            Opcode::MVN => self.execute_MVN(data_processing),
            Opcode::TST => self.execute_TST(data_processing),
            Opcode::TEQ => self.execute_TEQ(data_processing),
            _ => unreachable!()
        };
        data_processing.rd.value = Some(result);
        self.phys_reg_file.borrow_mut().set_value(data_processing.rd.phys_reg.unwrap(), result);

        rob_slot.renamed_registers.push(data_processing.rd.clone())
    }

    fn execute_CMP(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
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
    fn execute_TST(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();

        let result = rn_value & operand2_value;

        let zero_flag = result == 0;
        let negative_flag = (result & (1 << 63)) != 0;

        let mut rd_update = data_processing.rd_src.as_ref().unwrap().value.unwrap();

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

        rd_update
    }

    fn execute_TEQ(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();

        let result = rn_value ^ operand2_value;

        let zero_flag = result == 0;
        let negative_flag = (result & (1 << 63)) != 0;

        let mut rd_update = data_processing.rd_src.as_ref().unwrap().value.unwrap();

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

        rd_update
    }

    fn execute_SDIV(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();

        rn_value / operand2_value
    }

    fn execute_MOV(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        data_processing.operand2.value()
    }

    fn execute_MUL(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value.wrapping_mul(operand2_value)
    }

    fn execute_RSB(&mut self, data_processing: &mut RSDataProcessing) -> u64 {    // let rn = rs.source[0].value.unwrap();
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        operand2_value.wrapping_sub(rn_value)
    }

    fn execute_SUB(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value.wrapping_sub(operand2_value)
    }

    fn execute_ADD(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value.wrapping_add(operand2_value)
    }

    fn execute_AND(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value & operand2_value
    }

    fn execute_ORR(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value | operand2_value
    }

    fn execute_MVN(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        !rn_value
    }

    fn execute_EOR(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();
        let operand2_value = data_processing.operand2.value();
        rn_value ^ operand2_value
    }

    fn execute_NEG(&mut self, data_processing: &mut RSDataProcessing) -> DWordType {
        let rn_value = data_processing.rn.as_ref().unwrap().value.unwrap();

        rn_value.wrapping_neg()
    }

    fn execute_load_store(&mut self, load_store: &mut RSLoadStore, rob_slot: &mut ROBSlot) {
        match &load_store.opcode {
            Opcode::LDR => self.execute_LDR(load_store, rob_slot),
            Opcode::STR => self.execute_STR(load_store, rob_slot),
            _ => unreachable!()
        };
    }

    fn execute_STR(&mut self, load_store: &mut RSLoadStore, rob_slot: &mut ROBSlot) {
        let value = load_store.rd.value.unwrap();
        let address = load_store.rn.value.unwrap();

        let mut memory_subsystem = self.memory_subsystem.borrow_mut();
        memory_subsystem.sb.store(rob_slot.sb_pos.unwrap(), address, value);
    }

    fn execute_LDR(&mut self, load_store: &mut RSLoadStore, rob_slot: &mut ROBSlot) {
        let memory_subsystem = self.memory_subsystem.borrow_mut();
        let address = load_store.rn.value.unwrap() as usize;
        let value = memory_subsystem.memory[address];

        let rd = load_store.rd.phys_reg.unwrap();
        load_store.rd.value = Some(value);
        self.phys_reg_file.borrow_mut().set_value(rd, value);

        rob_slot.renamed_registers.push(load_store.rd.clone())
    }

    fn execute_branch(&mut self, branch: &mut RSBranch, rob_slot: &mut ROBSlot) {
        println!("opcode:{:?}", branch.opcode);
        match &branch.opcode {
            Opcode::B => self.execute_B(branch, rob_slot),
            Opcode::BL => self.execute_BL(branch, rob_slot),
            Opcode::BX => self.execute_BX(branch, rob_slot),
            Opcode::BEQ => self.execute_BEQ(branch, rob_slot),
            Opcode::BNE => self.execute_BNE(branch, rob_slot),
            Opcode::BGT => self.execute_BGT(branch, rob_slot),
            Opcode::BGE => self.execute_BGE(branch, rob_slot),
            Opcode::BLT => self.execute_BLT(branch, rob_slot),
            Opcode::BLE => self.execute_BLE(branch, rob_slot),
            Opcode::CBZ => self.execute_CBZ(branch, rob_slot),
            Opcode::CBNZ => self.execute_CBNZ(branch, rob_slot),
            Opcode::RET => self.execute_RET(branch, rob_slot),
            _ => unreachable!()
        };
    }

    fn execute_B(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        // // update the PC

        let branch_target = branch.target.value();
        rob_slot.branch_target_actual = branch_target as usize;
    }

    fn execute_BX(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        // // update the PC
        let branch_target = branch.target.value();
        rob_slot.branch_target_actual = branch_target as usize;
    }

    fn execute_BEQ(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let target = branch.target.value() as u64;
        let cpsr = branch.rt.as_ref().unwrap().value.unwrap();
        let pc = rob_slot.pc as DWordType;

        let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        let pc_update = if zero_flag == 1 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BNE(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let target = branch.target.value() as u64;
        let cpsr = branch.rt.as_ref().unwrap().value.unwrap();
        let pc = rob_slot.pc as DWordType;

        let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        let pc_update = if zero_flag == 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLT(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let target = branch.target.value() as u64;
        let cpsr = branch.rt.as_ref().unwrap().value.unwrap();
        let pc = rob_slot.pc as DWordType;

        let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;

        let pc_update = if negative_flag != overflow_flag { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLE(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let target = branch.target.value() as u64;
        let cpsr = branch.rt.as_ref().unwrap().value.unwrap();
        let pc = rob_slot.pc as DWordType;

        let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;

        let pc_update = if (zero_flag == 1) || (negative_flag != overflow_flag) {
            target
        } else {
            pc + 1
        };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGT(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let target = branch.target.value() as u64;
        let cpsr = branch.rt.as_ref().unwrap().value.unwrap();
        let pc = rob_slot.pc as DWordType;

        let zero_flag = (cpsr >> ZERO_FLAG) & 0x1;
        let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;

        let pc_update = if (zero_flag == 0) && (negative_flag == overflow_flag) {
            target
        } else {
            pc + 1
        };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGE(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let target = branch.target.value() as u64;
        let cpsr = branch.rt.as_ref().unwrap().value.unwrap();
        let pc = rob_slot.pc as DWordType;

        let negative_flag = (cpsr >> NEGATIVE_FLAG) & 0x1;
        let overflow_flag = (cpsr >> OVERFLOW_FLAG) & 0x1;

        let pc_update = if negative_flag == overflow_flag {
            target
        } else {
            pc + 1
        };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CBZ(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let reg_value = branch.rt.as_ref().unwrap().value.unwrap();
        let target = branch.target.value() as u64;
        let pc = rob_slot.pc as DWordType;

        let pc_update = if reg_value == 0 { target } else { pc + 1 };
        println!("pc_update {}", pc_update);
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CBNZ(&mut self, branch: &RSBranch, rob_slot: &mut ROBSlot) {
        let reg_value = branch.rt.as_ref().unwrap().value.unwrap();
        let target = branch.target.value() as u64;
        let pc = rob_slot.pc as DWordType;

        let pc_update = if reg_value != 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BL(&mut self, branch: &mut RSBranch, rob_slot: &mut ROBSlot) {
        let branch_target = branch.target.value();
        rob_slot.branch_target_actual = branch_target as usize;

        let pc_update = branch_target;

        // update LR
        let value = (rob_slot.pc + 1) as DWordType;
        let lr = branch.lr.as_mut().unwrap();
        lr.value = Some(value);
        self.phys_reg_file.borrow_mut().set_value(lr.phys_reg.unwrap(), value);
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_RET(&mut self, branch: &mut RSBranch, rob_slot: &mut ROBSlot) {
        // update the PC
        let branch_target = branch.target.value();
        let pc_update = branch_target;
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_ADR(&mut self, _rs: &mut RS, _rob_slot: &mut ROBSlot) {
        panic!("ADR is not implemented");
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
