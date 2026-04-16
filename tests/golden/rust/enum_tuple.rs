#[derive(Debug, Clone)]
pub enum Expr {
    Unit,
    Literal(i64),
    Add(Box<Expr>, Box<Expr>),
}
