    pub struct EU {
        pub index: u8,
        pub rs_index: u16,
        pub cycles_remaining: u8,
    }

    pub(crate) struct EUTable {
        pub capacity: u8,
        free_stack: Vec<u8>,
        array: Vec<EU>,
    }

    impl EUTable {
        pub(crate) fn new(capacity: u8) -> EUTable {
            let mut free_stack = Vec::with_capacity(capacity as usize);
            let mut array = Vec::with_capacity(capacity as usize);
            for i in 0..capacity {
                array.push(EU { index: i, cycles_remaining: 0, rs_index: 0 });
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
            return self.array.get_mut(eu_index as usize).unwrap();
        }

        pub(crate) fn allocate(&mut self) -> u8 {
            if let Some(last_element) = self.free_stack.pop() {
                return last_element;
            } else {
                panic!("No free PhysReg")
            }
        }

        pub(crate) fn deallocate(&mut self, eu_index: u8) {
            self.free_stack.push(eu_index);
        }
    }
