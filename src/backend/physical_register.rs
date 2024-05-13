use crate::instructions::instructions::{RegisterType, WordType};

pub struct PhysRegEntry {
    pub(crate) value: WordType,
    pub(crate) has_value: bool,
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
            entries.push(PhysRegEntry { value: 0, has_value: false });
            free_stack.push(count - 1 - i);
        }

        PhysRegFile { count, entries, free_stack }
    }

    pub(crate) fn get(&self, reg: RegisterType) -> &PhysRegEntry {
        return self.entries.get(reg as usize).unwrap();
    }

    pub(crate) fn get_mut(&mut self, reg: RegisterType) -> &mut PhysRegEntry {
        return self.entries.get_mut(reg as usize).unwrap();
    }

    pub(crate) fn allocate(&mut self) -> RegisterType {
        if let Some(reg) = self.free_stack.pop() {
            let phys_reg_entry = self.get(reg);
            assert!(!phys_reg_entry.has_value, " The allocated physical register {} should not have a value", reg);
            return reg;
        } else {
            panic!("No free PhysReg")
        }
    }

    pub(crate) fn deallocate(&mut self, reg: RegisterType) {
        if self.free_stack.contains(&reg) {
            panic!("Phys register {} can be deallocated while it is still on the free stack", reg);
        }

        let phys_reg_entry = self.get(reg);
        assert!(!phys_reg_entry.has_value, " The deallocated physical register {} should not a value!", reg);


        self.free_stack.push(reg);
    }
}


