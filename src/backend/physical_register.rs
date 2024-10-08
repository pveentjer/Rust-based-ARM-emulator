use crate::instructions::instructions::{DWordType, RegisterType};

#[derive(Clone, Copy, PartialEq, Debug)]
enum PhysRegEntryState {
    IDLE,
    BUSY,
}

pub(crate) struct PhysRegEntry {
    pub(crate) value: DWordType,
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

    pub(crate) fn set_value(&mut self, reg: RegisterType, value: DWordType) {
        let entry = self.get_mut(reg);
        debug_assert!(!entry.has_value);
        entry.has_value = true;
        entry.value = value;
    }

    pub(crate) fn get_value(&self, reg: RegisterType) -> DWordType {
        let entry = self.get(reg);
        debug_assert!(entry.has_value);
        entry.value
    }

    pub(crate) fn allocate(&mut self) -> RegisterType {
        if let Some(reg) = self.free_stack.pop() {
            let entry = self.entries.get_mut(reg as usize).unwrap();
            debug_assert!(entry.state == PhysRegEntryState::IDLE);
            debug_assert!(!entry.has_value, " The allocated physical register {} should not have a value", reg);
            entry.state = PhysRegEntryState::BUSY;
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
        debug_assert!(!self.free_stack.contains(&reg), "Phys register {} can't be deallocated while it is also on the free stack", reg);

        let entry = self.get_mut(reg);

        debug_assert!(entry.state == PhysRegEntryState::BUSY);

        entry.reset();

        self.free_stack.push(reg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate() {
        let mut reg_file = PhysRegFile::new(256);
        let reg = reg_file.allocate();
        assert_eq!(reg, 0);

        let entry = reg_file.get(reg);
        assert_eq!(entry.state, PhysRegEntryState::BUSY);
        assert_eq!(entry.has_value, false);
        assert_eq!(entry.value, 0);
    }
}
