use crate::backend::execution_unit::EUState::IDLE;

/// A single execution unit.
pub(crate) struct EU {
    pub(crate) index: u8,
    pub(crate) rs_index: Option<u16>,
    pub(crate) cycles_remaining: u8,
    pub(crate) state: EUState,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum EUState {
    IDLE,
    BUSY,
}

impl EU {
    fn reset(&mut self) {
        self.rs_index = None;
        self.cycles_remaining = 0;
        self.state = IDLE;
    }
}

/// The table containing all execution units of a CPU core.
pub(crate) struct EUTable {
    pub(crate) capacity: u8,
    free_stack: Vec<u8>,
    array: Vec<EU>,
}

impl EUTable {
    pub(crate) fn new(capacity: u8) -> EUTable {
        let mut free_stack = Vec::with_capacity(capacity as usize);
        let mut array = Vec::with_capacity(capacity as usize);
        for i in 0..capacity {
            array.push(EU {
                index: i,
                cycles_remaining: 0,
                rs_index: None,
                state: IDLE});
            free_stack.push(i);
        }

        EUTable {
            capacity,
            array,
            free_stack,
        }
    }

    pub(crate) fn has_free(&self) -> bool {
        return !self.free_stack.is_empty();
    }

    pub(crate) fn get_mut(&mut self, eu_index: u8) -> &mut EU {
        self.array.get_mut(eu_index as usize).unwrap()
    }

    pub(crate) fn allocate(&mut self) -> u8 {
        if let Some(last_element) = self.free_stack.pop() {
            let eu = self.array.get_mut(last_element as usize).unwrap();
            debug_assert!(eu.state == EUState::IDLE);
            debug_assert!(eu.rs_index.is_none());
            debug_assert!(eu.cycles_remaining == 0);

            eu.state = EUState::BUSY;
            return last_element;
        } else {
            panic!("No free PhysReg")
        }
    }

    pub(crate) fn deallocate(&mut self, eu_index: u8) {
        let eu = self.array.get_mut(eu_index as usize).unwrap();
        debug_assert!(eu.state == EUState::BUSY);
        debug_assert!(eu.rs_index.is_some());
        debug_assert!(!self.free_stack.contains(&eu_index));

        eu.reset();
        self.free_stack.push(eu_index);
    }
}
