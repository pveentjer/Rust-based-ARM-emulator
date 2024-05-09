use std::rc::Rc;
use crate::instructions::{create_ADD, create_INC, create_LOAD, create_NOP, create_PRINTR, create_STORE, Instr, Program};

// for the time being we always return the same program
pub(crate) fn load(_: &str) -> Program {
    let mut code = Vec::<Rc<Instr>>::new();
    let mut line = 0;
    //
    // line += 1;
    // code.push(Rc::new(create_LOAD(0, 0, line)));
    //
    // for _ in 0..10 {
    //     line += 1;
    //     code.push(Rc::new(create_NOP(line)));
    // }

    for _ in 0.. 10 {
        line += 1;
        code.push(Rc::new(create_INC(0, line)));
    }

    //
    // for _ in 0..10 {
    //     line += 1;
    //     code.push(Rc::new(create_NOP(line)));
    // }
    //
    // line += 1;
    // code.push(Rc::new(create_INC(0,line) ));

    // for _ in 0..10 {
    //     line += 1;
    //     code.push(Rc::new(create_NOP(line)));
    // }
    //
    // line += 1;
    // code.push(Rc::new(create_PRINTR(0, line)));


    // line+=1;
    // code.push(Rc::new(create_NOP(line)));
    // line+=1;
    // code.push(Rc::new(create_NOP(line)));
    // line+=1;
    // code.push(Rc::new(create_NOP(line)));
    // line+=1;
    // code.push(Rc::new(create_NOP(line)));

    // code.push(Rc::new(create_STORE(2, 2)));


    return Program::new(code);
}