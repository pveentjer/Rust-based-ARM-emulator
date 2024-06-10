use crate::instructions::instructions::RegisterType;

pub(crate) struct RATEntry {
    // the physical register in the arch register to phys register mapping
    pub(crate) phys_reg: RegisterType,
    // if the entry currently contains a valid architectural to physical register mapping
    pub(crate) valid: bool,
}

/// The Register Alias Table. This structure is used for the register
/// renaming process. The RAT entry for a given architectural register
/// points to the physical register to use. As long as such a entry
/// exists, it should be used.
pub(crate) struct RAT {
    pub(crate) table: Vec<RATEntry>,
}

impl RAT {
    pub(crate) fn new(phys_reg_count: u16) -> Self {
        let mut table = Vec::with_capacity(phys_reg_count as usize);
        for _ in 0..phys_reg_count {
            table.push(RATEntry { phys_reg: 0, valid: false });
        }
        Self { table }
    }

    pub(crate) fn flush(&mut self) {
        for k in 0..self.table.len() {
            let option = self.table.get_mut(k).unwrap();
            option.valid = false;
        }
    }

    pub(crate) fn get(&self, arch_reg: RegisterType) -> &RATEntry {
        return self.table.get(arch_reg as usize).unwrap();
    }

    pub(crate) fn get_mut(&mut self, arch_reg: RegisterType) -> &mut RATEntry {
        return self.table.get_mut(arch_reg as usize).unwrap();
    }
}
