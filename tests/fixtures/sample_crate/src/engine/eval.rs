//! Expression evaluation engine.

use crate::models::Value;
use super::EngineError;

use std::collections::HashMap;

/// Evaluation context holding variable bindings
pub struct EvalContext<'a> {
    pub scope: &'a HashMap<String, Value>,
    pub depth: usize,
    max_depth: usize,
}

/// Something that can be evaluated
pub trait Evaluable {
    /// Evaluate this expression in the given context
    fn eval(&self, ctx: &mut EvalContext) -> Result<Value, EngineError>;
}

/// A simple expression type
pub enum Expr {
    /// A literal value
    Literal(Value),
    /// A variable reference
    Variable(String),
    /// A binary operation
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

/// Binary operators
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl Evaluable for Expr {
    fn eval(&self, ctx: &mut EvalContext) -> Result<Value, EngineError> {
        ctx.check_depth()?;
        match self {
            Expr::Literal(v) => Ok(v.clone()),
            Expr::Variable(name) => resolve_name(name, ctx.scope),
            Expr::BinOp { op, left, right } => {
                let l = left.eval(ctx)?;
                let r = right.eval(ctx)?;
                apply_operator(op, &l, &r)
            }
        }
    }
}

impl<'a> EvalContext<'a> {
    /// Create a new evaluation context
    pub fn new(scope: &'a HashMap<String, Value>) -> Self {
        EvalContext {
            scope,
            depth: 0,
            max_depth: 100,
        }
    }

    /// Set the maximum recursion depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    fn check_depth(&self) -> Result<(), EngineError> {
        if self.depth >= self.max_depth {
            Err(EngineError::StackOverflow)
        } else {
            Ok(())
        }
    }
}

/// Evaluate an expression in the given context
pub fn evaluate(expr: &Expr, ctx: &mut EvalContext) -> Result<Value, EngineError> {
    expr.eval(ctx)
}

/// Evaluate a batch of expressions
pub fn evaluate_batch(
    exprs: &[Expr],
    ctx: &mut EvalContext,
) -> Result<Vec<Value>, EngineError> {
    exprs.iter().map(|e| e.eval(ctx)).collect()
}

fn resolve_name(name: &str, scope: &HashMap<String, Value>) -> Result<Value, EngineError> {
    scope
        .get(name)
        .cloned()
        .ok_or_else(|| EngineError::UnknownVariable(name.to_string()))
}

fn apply_operator(op: &BinOp, left: &Value, right: &Value) -> Result<Value, EngineError> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => match op {
            BinOp::Add => Ok(Value::Int(l + r)),
            BinOp::Sub => Ok(Value::Int(l - r)),
            BinOp::Mul => Ok(Value::Int(l * r)),
            BinOp::Div => {
                if *r == 0 {
                    Err(EngineError::DivisionByZero)
                } else {
                    Ok(Value::Int(l / r))
                }
            }
        },
        _ => Ok(Value::Null),
    }
}
