use std::rc::Rc;
use crate::instructions::{create_ADD, create_LOAD, create_STORE, Instr, Program};

// for the time being we always return the same program
pub(crate) fn load(_: &str) -> Program {
    let mut code = Vec::<Rc<Instr>>::new();
    code.push(Rc::new(create_LOAD(0, 0)));
    code.push(Rc::new(create_LOAD(1, 1)));
    code.push(Rc::new(create_ADD(0, 1, 2)));
    code.push(Rc::new(create_STORE(2, 2)));

    return Program { code };
}