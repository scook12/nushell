use crate::evaluate::{evaluate_expr, Scope};
use crate::parser::lexer::Spanned;
use crate::prelude::*;
use indexmap::IndexMap;

#[allow(unused)]
#[derive(Debug)]
pub enum NamedType {
    Switch,
    Mandatory(NamedValue),
    Optional(NamedValue),
}

#[derive(Debug)]
pub enum NamedValue {
    Single,
    Tuple,

    #[allow(unused)]
    Block,

    #[allow(unused)]
    Array,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum PositionalType {
    Value(String),
    Block(String),
}

impl PositionalType {
    crate fn name(&self) -> String {
        match self {
            PositionalType::Value(s) => s.clone(),
            PositionalType::Block(s) => s.clone(),
        }
    }

    crate fn evaluate(
        &self,
        arg: ast::Expression,
        scope: &Scope,
    ) -> Result<Spanned<Value>, ShellError> {
        match self {
            PositionalType::Value(_) => evaluate_expr(&arg, scope),
            PositionalType::Block(_) => match arg {
                ast::Expression {
                    expr: ast::RawExpression::Block(b),
                    ..
                } => Ok(Spanned::from_item(Value::block(b.expr), arg.span.clone())),
                ast::Expression {
                    expr: ast::RawExpression::Binary(binary),
                    ..
                } => {
                    // TODO: Use original spans
                    let mut b = ast::ExpressionBuilder::new();
                    if let Some(s) = binary.left.as_string() {
                        Ok(Spanned::from_item(
                            Value::block(b.binary((
                                &|b| b.path((&|b| b.var("it"), vec![s.clone()])),
                                &|_| binary.operator.clone(),
                                &|_| binary.right.clone(),
                            ))),
                            arg.span.clone(),
                        ))
                    } else {
                        let mut b = ast::ExpressionBuilder::new();
                        let expr = b.binary((
                            &|_| binary.left.clone(),
                            &|_| binary.operator.clone(),
                            &|_| binary.right.clone(),
                        ));

                        Ok(Spanned::from_item(Value::block(expr), arg.span.clone()))
                    }
                }
                other => {
                    let span = other.span.clone();
                    Ok(Spanned::from_item(Value::block(other), span))
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct CommandConfig {
    crate name: String,
    crate mandatory_positional: Vec<PositionalType>,
    crate optional_positional: Vec<PositionalType>,
    crate rest_positional: bool,
    crate named: IndexMap<String, NamedType>,
}

#[derive(Debug, Default)]
pub struct Args {
    pub positional: Vec<Spanned<Value>>,
    pub named: IndexMap<String, Value>,
}

impl CommandConfig {
    crate fn evaluate_args(
        &self,
        args: impl Iterator<Item = &'expr ast::Expression>,
        scope: &Scope,
    ) -> Result<Args, ShellError> {
        let mut positional: Vec<Spanned<Value>> = vec![];
        let mut named: IndexMap<String, Value> = IndexMap::default();

        let mut args: Vec<ast::Expression> = args.cloned().collect();

        for (key, ty) in self.named.iter() {
            let index = args.iter().position(|a| a.is_flag(&key));

            match (index, ty) {
                (Some(i), NamedType::Switch) => {
                    args.remove(i);
                    named.insert(key.clone(), Value::boolean(true));
                }

                (None, NamedType::Switch) => {}

                (Some(i), NamedType::Optional(v)) => {
                    args.remove(i);
                    named.insert(key.clone(), extract_named(&mut args, i, v)?);
                }

                (None, NamedType::Optional(_)) => {}

                (Some(i), NamedType::Mandatory(v)) => {
                    args.remove(i);
                    named.insert(key.clone(), extract_named(&mut args, i, v)?);
                }

                (None, NamedType::Mandatory(_)) => {
                    return Err(ShellError::string(&format!(
                        "Expected mandatory argument {}, but it was missing",
                        key
                    )))
                }
            }
        }

        let mut args = args.into_iter();

        for param in &self.mandatory_positional {
            let arg = args.next();

            let value = match arg {
                None => {
                    return Err(ShellError::string(format!(
                        "expected mandatory positional argument {}",
                        param.name()
                    )))
                }

                Some(arg) => param.evaluate(arg.clone(), scope)?,
            };

            positional.push(value);
        }

        if self.rest_positional {
            let rest: Result<Vec<Spanned<Value>>, _> =
                args.map(|i| evaluate_expr(&i, &Scope::empty())).collect();
            positional.extend(rest?);
        } else {
            let rest: Vec<ast::Expression> = args.collect();

            if rest.len() > 0 {
                return Err(ShellError::string(&format!(
                    "Too many arguments, extras: {:?}",
                    rest
                )));
            }
        }

        Ok(Args { positional, named })
    }

    #[allow(unused)]
    crate fn signature(&self) -> String {
        format!("TODO")
    }
}

fn extract_named(
    v: &mut Vec<ast::Expression>,
    position: usize,
    ty: &NamedValue,
) -> Result<Value, ShellError> {
    match ty {
        NamedValue::Single => {
            let expr = v.remove(position);
            expect_simple_expr(expr)
        }

        NamedValue::Tuple => {
            let expr = v.remove(position);
            let next = v.remove(position);

            let list = vec![expect_simple_expr(expr)?, expect_simple_expr(next)?];
            Ok(Value::List(list))
        }

        other => Err(ShellError::string(&format!(
            "Unimplemented named argument {:?}",
            other
        ))),
    }
}

fn expect_simple_expr(expr: ast::Expression) -> Result<Value, ShellError> {
    match &*expr {
        ast::RawExpression::Leaf(l) => Ok(match l {
            ast::Leaf::Bare(s) => Value::string(s.to_string()),
            ast::Leaf::String(s) => Value::string(s),
            ast::Leaf::Boolean(b) => Value::boolean(*b),
            ast::Leaf::Int(i) => Value::int(*i),
            ast::Leaf::Unit(i, unit) => unit.compute(*i),
        }),

        // TODO: Diagnostic
        other => Err(ShellError::string(&format!(
            "Expected a value, found {}",
            other.print()
        ))),
    }
}

pub trait CommandRegistry {
    fn get(&self, name: &str) -> CommandConfig;
}