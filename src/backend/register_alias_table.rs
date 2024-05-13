use crate::instructions::RegisterType;

pub struct RATEntry {
    pub(crate) phys_reg: RegisterType,
    // The number of pending writes; if 0, then the entry is not valid
    pub(crate) valid: bool,
}

pub(crate) struct RAT {
    pub(crate) table: Vec<RATEntry>,
}

impl RAT {
    pub fn new(phys_reg_count: u16) -> Self {
        let mut table = Vec::with_capacity(phys_reg_count as usize);
        for _ in 0..phys_reg_count {
            table.push(RATEntry { phys_reg: 0, valid: false });
        }
        Self { table }
    }

    pub fn get(&self, arch_reg: RegisterType) -> &RATEntry {
        return self.table.get(arch_reg as usize).unwrap();
    }

    pub fn get_mut(&mut self, arch_reg: RegisterType) -> &mut RATEntry {
        return self.table.get_mut(arch_reg as usize).unwrap();
    }
}
