#[derive(PartialEq, Eq, Hash, Debug)]
pub enum ast_Expr {
    Number(i32),
    Identifier(String),
    Op(Box<ast_Expr>, ast_Opcode, Box<ast_Expr>),
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum ast_Opcode {
    Mul,
    Div,
    Add,
    Sub,
}

pub struct ast_Program {
    pub statements: Vec<ast_Statement>,
}

impl ast_Program {
    pub fn new(statements: Vec<ast_Statement>) -> Self {
        Self { statements }
    }
}

pub struct ast_StatementBody {
    pub identifier: String,
    pub expression: Box<ast_Expr>,
}

pub enum ast_Statement {
    Assignment(ast_StatementBody),
    Definition(ast_StatementBody),
}

impl ast_Statement {
    pub fn new_assignment(identifier: String, expression: Box<ast_Expr>) -> Self {
        Self::Assignment(ast_StatementBody {
            identifier,
            expression,
        })
    }

    pub fn new_definition(identifier: String, expression: Box<ast_Expr>) -> Self {
        Self::Definition(ast_StatementBody {
            identifier,
            expression,
        })
    }
}
