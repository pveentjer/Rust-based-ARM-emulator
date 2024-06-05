use crate::instructions::instructions::{RegisterType, WordType};

#[derive(Clone, Copy, PartialEq, Debug)]
enum PhysRegEntryState {
    IDLE,
    BUSY,
}

pub(crate) struct PhysRegEntry {
    pub(crate) value: WordType,
    pub(crate) has_value: bool,
    state: PhysRegEntryState,
}

impl PhysRegEntry {
    fn reset(&mut self) {
        self.value = 0;
        self.has_value = false;
        self.state = PhysRegEntryState::IDLE;
    }
}

pub(crate) struct PhysRegFile {
    free_stack: Vec<u16>,
    count: u16,
    entries: Vec<PhysRegEntry>,
}

impl PhysRegFile {
    pub(crate) fn new(count: u16) -> PhysRegFile {
        let mut free_stack = Vec::with_capacity(count as usize);
        let mut entries = Vec::with_capacity(count as usize);
        for i in 0..count {
            entries.push(PhysRegEntry {
                value: 0,
                has_value: false,
                state: PhysRegEntryState::IDLE,
            });
            free_stack.push(count - 1 - i);
        }

        PhysRegFile { count, entries, free_stack }
    }

    pub(crate) fn get(&self, reg: RegisterType) -> &PhysRegEntry {
        let entry = self.entries.get(reg as usize).unwrap();
        debug_assert!(entry.state == PhysRegEntryState::BUSY, "phys register {} is not in busy state", reg);
        return entry;
    }

    pub(crate) fn get_mut(&mut self, reg: RegisterType) -> &mut PhysRegEntry {
        let entry = self.entries.get_mut(reg as usize).unwrap();
        debug_assert!(entry.state == PhysRegEntryState::BUSY, "phys register {} is not in busy state", reg);
        return entry;
    }

    pub(crate) fn allocate(&mut self) -> RegisterType {
        if let Some(reg) = self.free_stack.pop() {
            let entry = self.entries.get_mut(reg as usize).unwrap();
            debug_assert!(entry.state == PhysRegEntryState::IDLE);
            debug_assert!(!entry.has_value, " The allocated physical register {} should not have a value", reg);
            entry.state = PhysRegEntryState::BUSY;
            //println!("Phys Register: allocate {}",reg);
            return reg;
        } else {
            panic!("No free PhysReg")
        }
    }

    pub(crate) fn flush(&mut self) {
        self.free_stack.clear();

        for i in 0..self.count {
            let entry = &mut self.entries[i as usize];

            self.free_stack.push(i);
            entry.reset();
        }
    }

    pub(crate) fn deallocate(&mut self, reg: RegisterType) {
       // println!("Phys Register: deallocate {}",reg);

        debug_assert!(!self.free_stack.contains(&reg), "Phys register {} can't be deallocated while it is also on the free stack", reg);

        let entry = self.get_mut(reg);

        debug_assert!(entry.state == PhysRegEntryState::BUSY);
        debug_assert!(!entry.has_value, " The deallocated physical register {} should not have a value", reg);

        entry.reset();

        self.free_stack.push(reg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate() {
        let mut regs = PhysRegFile::new(256);
        let reg = regs.allocate();
        assert_eq!(reg, 0);

        let entry = regs.get_mut(reg);
        assert_eq!(entry.state, PhysRegEntryState::BUSY);
    }
}
