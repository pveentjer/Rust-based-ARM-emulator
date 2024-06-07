use std::cell::RefCell;
use std::rc::Rc;
use crate::backend::reorder_buffer::ROBSlot;
use crate::backend::reservation_station::RS;
use crate::cpu::{CARRY_FLAG, CPUConfig, NEGATIVE_FLAG, OVERFLOW_FLAG, ZERO_FLAG};
use crate::instructions::instructions::{DWordType, Opcode, Operand};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;

/// A single execution unit.
pub(crate) struct EU {
    pub(crate) index: u8,
    pub(crate) rs_index: Option<u16>,
    pub(crate) cycles_remaining: u8,
    pub(crate) state: EUState,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
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

        if self.trace {
            let instr = rob_slot.instr.as_ref().unwrap();
            println!("Executing {}", instr);
        }

        match rs.opcode {
            Opcode::NOP => {}
            Opcode::ADD => self.execute_ADD(rs, rob_slot),
            Opcode::SUB => self.execute_SUB(rs, rob_slot),
            Opcode::MUL => self.execute_MUL(rs, rob_slot),
            Opcode::SDIV => self.execute_SDIV(rs, rob_slot),
            Opcode::NEG => self.execute_NEG(rs, rob_slot),
            Opcode::AND => self.execute_AND(rs, rob_slot),
            Opcode::MOV => self.execute_MOV(rs, rob_slot),
            Opcode::ADR => self.execute_ADR(rs, rob_slot),
            Opcode::ORR => self.execute_ORR(rs, rob_slot),
            Opcode::EOR => self.execute_EOR(rs, rob_slot),
            Opcode::MVN => self.execute_MVN(rs, rob_slot),
            Opcode::LDR => self.execute_LDR(rs, rob_slot),
            Opcode::STR => self.execute_STR(rs, rob_slot),
            Opcode::PRINTR => self.execute_PRINTR(rs, rob_slot),
            Opcode::CMP => self.execute_CMP(rs, rob_slot),
            Opcode::BEQ => self.execute_BEQ(rs, rob_slot),
            Opcode::BNE => self.execute_BNE(rs, rob_slot),
            Opcode::BLT => self.execute_BLT(rs, rob_slot),
            Opcode::BLE => self.execute_BLE(rs, rob_slot),
            Opcode::BGT => self.execute_BGT(rs, rob_slot),
            Opcode::BGE => self.execute_BGE(rs, rob_slot),
            Opcode::CBZ => self.execute_CBZ(rs, rob_slot),
            Opcode::CBNZ => self.execute_CBNZ(rs, rob_slot),
            Opcode::B => self.execute_B(rs, rob_slot),
            Opcode::BX => self.execute_BX(rs, rob_slot),
            Opcode::BL => self.execute_BL(rs, rob_slot),
            Opcode::EXIT => {}
            Opcode::DSB => {}
        }
    }

    fn execute_BEQ(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr == 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BNE(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr != 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLT(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr < 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLE(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr <= 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGT(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr > 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGE(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr >= 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CBZ(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let reg_value = rs.source[0].get_immediate();
        let branch = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if reg_value == 0 { branch } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CBNZ(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let reg_value = rs.source[0].get_immediate();
        let branch = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if reg_value != 0 { branch } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BL(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let branch_target = rs.source[0].get_code_address();

        let pc_update = branch_target;

        // update LR
        rob_slot.result.push((rob_slot.pc + 1) as DWordType);
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BX(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // update the PC
        let branch_target = rs.source[0].get_immediate() as i64;
        let pc_update = branch_target;
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_B(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        // update the PC
        let branch_target = rs.source[0].get_code_address();
        let pc_update = branch_target;
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CMP(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
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

        rob_slot.result.push(new_cprs_value as u64);
    }

    fn execute_PRINTR(&mut self, rs: &mut RS,rob_slot: &mut ROBSlot) {
        let instr = rob_slot.instr.as_ref().unwrap();

        println!("PRINTR {}={}", Operand::Register(instr.source[0].get_register()), rs.source[0].get_immediate());
    }

    fn execute_STR(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        println!("execute STR");
        rob_slot.result.push(rs.source[0].get_immediate())
    }

    fn execute_LDR(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let memory_subsystem = self.memory_subsystem.borrow_mut();
        rob_slot.result.push(memory_subsystem.memory[rs.source[0].get_immediate() as usize])
    }

    fn execute_MVN(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(!rs.source[0].get_immediate())
    }

    fn execute_MOV(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate())
    }

    fn execute_ADR(&mut self, _rs: &mut RS, _rob_slot: &mut ROBSlot) {
        panic!("ADR is not implemented");
    }

    fn execute_EOR(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let operand2 = rs.source[1].get_immediate();
        let rd = rn ^ operand2;
        rob_slot.result.push(rd)
    }

    fn execute_ORR(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let operand2 = rs.source[1].get_immediate();
        let rd = rn | operand2;
        rob_slot.result.push(rd)
    }

    fn execute_AND(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let operand2 = rs.source[1].get_immediate();
        let rd = rn & operand2;
        rob_slot.result.push(rd)
    }

    fn execute_NEG(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let rd = rn.wrapping_neg();
        rob_slot.result.push(rd)
    }

    fn execute_SDIV(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let operand2 = rs.source[1].get_immediate();
        let rd = rn / operand2;
        rob_slot.result.push(rd)
    }

    fn execute_MUL(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let operand2 = rs.source[1].get_immediate();
        let rd = rn.wrapping_mul(operand2);
        rob_slot.result.push(rd)
    }

    fn execute_SUB(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let operand2 = rs.source[1].get_immediate();
        let rd = rn.wrapping_sub(operand2);
        rob_slot.result.push(rd)
    }

    fn execute_ADD(&mut self, rs: &mut RS, rob_slot: &mut ROBSlot) {
        let rn = rs.source[0].get_immediate();
        let operand2 = rs.source[1].get_immediate();
        let rd = rn.wrapping_add(operand2);
        rob_slot.result.push(rd)
    }
}

/// The table containing all execution units of a CPU core.
pub(crate) struct EUTable {
    pub(crate) capacity: u8,
    idle_stack: Vec<u8>,
    array: Vec<EU>,
}

impl EUTable {
    pub(crate) fn new(cpu_config: &CPUConfig,
                      memory_subsystem: Rc<RefCell<MemorySubsystem>>) -> EUTable {
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
                memory_subsystem: Rc::clone(&memory_subsystem),
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
