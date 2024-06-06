use std::cell::RefMut;
use std::rc::Rc;
use crate::backend::reorder_buffer::ROBSlot;
use crate::backend::reservation_station::RS;
use crate::cpu::{CARRY_FLAG, CPUConfig, NEGATIVE_FLAG, OVERFLOW_FLAG, ZERO_FLAG};
use crate::instructions::instructions::{DWordType, Instr, Opcode, Operand};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;

/// A single execution unit.
pub(crate) struct EU {
    pub(crate) index: u8,
    pub(crate) rs_index: Option<u16>,
    pub(crate) cycles_remaining: u8,
    pub(crate) state: EUState,
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
                 memory_subsystem: &mut RefMut<MemorySubsystem>,
                 rs: &mut RS,
                 rob_slot: &mut ROBSlot,
                 instr: Rc<Instr>) {
        debug_assert!(self.state == EUState::EXECUTING);
        debug_assert!(self.cycles_remaining > 0);

        self.cycles_remaining -= 1;
        if self.cycles_remaining > 0 {
            // the execution unit isn't finished with its work
            return;
        }
        self.state = EUState::COMPLETED;

        if self.trace {
            println!("Executing {}", instr);
        }

        match rs.opcode {
            Opcode::NOP => {}
            Opcode::ADD => Self::execute_ADD(rs, rob_slot),
            Opcode::SUB => Self::execute_SUB(rs, rob_slot),
            Opcode::MUL => Self::execute_mul(rs, rob_slot),
            Opcode::SDIV => Self::execute_SDIV(rs, rob_slot),
            Opcode::NEG => Self::execute_NEG(rs, rob_slot),
            Opcode::AND => Self::execute_AND(rs, rob_slot),
            Opcode::MOV => Self::execute_MOV(rs, rob_slot),
            Opcode::ADR => {
                //todo
            }
            Opcode::ORR => Self::execute_ORR(rs, rob_slot),
            Opcode::EOR => Self::execute_EOR(rs, rob_slot),
            Opcode::MVN => Self::execute_MVN(rs, rob_slot),
            Opcode::LDR => Self::execute_LDR(memory_subsystem, rs, rob_slot),
            Opcode::STR => Self::execute_STR(rs, rob_slot),
            Opcode::PRINTR => Self::execute_PRINTR(rs, &instr),
            Opcode::CMP => Self::execute_CMP(rs, rob_slot),
            Opcode::BEQ => Self::execute_BEQ(rs, rob_slot),
            Opcode::BNE => Self::execute_BNE(rs, rob_slot),
            Opcode::BLT => Self::execute_BLT(rs, rob_slot),
            Opcode::BLE => Self::execute_BLE(rs, rob_slot),
            Opcode::BGT => Self::execute_BGT(rs, rob_slot),
            Opcode::BGE => Self::execute_BGE(rs, rob_slot),
            Opcode::CBZ => Self::execute_CBZ(rs, rob_slot),
            Opcode::CBNZ => Self::execute_CBNZ(rs, rob_slot),
            Opcode::B => Self::execute_B(rs, rob_slot),
            Opcode::BX => Self::execute_BX(rs, rob_slot),
            Opcode::BL => Self::execute_BL(rs, rob_slot),
            Opcode::EXIT => {}
            Opcode::DSB => {}
        }
    }

    fn execute_BEQ(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr == 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BNE(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr != 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLT(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr < 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BLE(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr <= 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGT(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr > 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BGE(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let target = rs.source[0].get_immediate();
        let cpsr = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if cpsr >= 0 { target } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }


    fn execute_CBZ(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let reg_value = rs.source[0].get_immediate();
        let branch = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if reg_value == 0 { branch } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CBNZ(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let reg_value = rs.source[0].get_immediate();
        let branch = rs.source[1].get_code_address();
        let pc = rob_slot.pc as DWordType;

        let pc_update = if reg_value != 0 { branch } else { pc + 1 };
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BL(rs: &mut RS, rob_slot: &mut ROBSlot) {
        let branch_target = rs.source[0].get_code_address();

        let pc_update = branch_target;

        // update LR
        rob_slot.result.push((rob_slot.pc + 1) as DWordType);
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_BX(rs: &mut RS, rob_slot: &mut ROBSlot) {
        // update the PC
        let branch_target = rs.source[0].get_immediate() as i64;
        let pc_update = branch_target;
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_B(rs: &mut RS, rob_slot: &mut ROBSlot) {
        // update the PC
        let branch_target = rs.source[0].get_code_address();
        let pc_update = branch_target;
        rob_slot.branch_target_actual = pc_update as usize;
    }

    fn execute_CMP(rs: &mut RS, rob_slot: &mut ROBSlot) {
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

    fn execute_PRINTR(rs: &mut RS, instr: &Rc<Instr>) {
        println!("PRINTR {}={}", Operand::Register(instr.source[0].get_register()), rs.source[0].get_immediate());
    }

    fn execute_STR(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate())
    }

    fn execute_LDR(mut memory_subsystem: &mut RefMut<MemorySubsystem>, rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(memory_subsystem.memory[rs.source[0].get_immediate() as usize])
    }

    fn execute_MVN(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(!rs.source[0].get_immediate())
    }

    fn execute_EOR(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate() ^ rs.source[1].get_immediate())
    }

    fn execute_ORR(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate() | rs.source[1].get_immediate())
    }

    fn execute_MOV(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate())
    }

    fn execute_AND(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate() & rs.source[1].get_immediate())
    }

    fn execute_NEG(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(-rs.source[0].get_immediate())
    }

    fn execute_SDIV(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate() / rs.source[1].get_immediate())
    }

    fn execute_mul(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate() * rs.source[1].get_immediate())
    }

    fn execute_SUB(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate() - rs.source[1].get_immediate())
    }

    fn execute_ADD(rs: &mut RS, rob_slot: &mut ROBSlot) {
        rob_slot.result.push(rs.source[0].get_immediate() + rs.source[1].get_immediate())
    }
}

/// The table containing all execution units of a CPU core.
pub(crate) struct EUTable {
    pub(crate) capacity: u8,
    idle_stack: Vec<u8>,
    array: Vec<EU>,
}

impl EUTable {
    pub(crate) fn new(cpu_config: &CPUConfig) -> EUTable {
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
        // println!("EUTable has_idle: {}",!self.idle_stack.is_empty());

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
