mod errors;
mod tokens;
mod parse;
mod util;
mod validate;
mod exp;

#[cfg(test)]
pub use self::parse::test;

use typed_arena::Arena;
use vm::{Symbol, SymbolIntern};
use compiler::parse::tokens::Position;
use util::iterators_same;

pub use self::errors::ParseError;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Span {
    start: Position,
    end: Position,
}

impl Span {
    pub fn dummy() -> Span {
        Span {
            start: Position(0, 0),
            end: Position(0, 0),
        }
    }
    fn from_pos(p1: Position, p2: Position) -> Span {
        if p1 < p2 {
            Span {
                start: p1,
                end: p2,
            }
        } else {
            Span {
                start: p2,
                end: p1,
            }
        }
    }

    fn join(self, other: Span) -> Span {
        use std::cmp::{min, max};
        let lowest = min(self.start, other.start);
        let highest = max(self.end, other.end);
        Span {
            start: lowest,
            end: highest,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ast<'ast> {
    BoolLit(bool, Span),
    StringLit(String, Span),
    IntLit(i64, Span),
    FloatLit(f64, Span),
    ListLit(Vec<&'ast Ast<'ast>>, Span),
    MapLit(Vec<(&'ast Ast<'ast>, &'ast Ast<'ast>)>, Span),
    Symbol(Symbol, Span),
    Add(Vec<&'ast Ast<'ast>>, Span),
    Quote(&'ast Ast<'ast>, Span),
    List(Vec<&'ast Ast<'ast>>, Span),
    If(&'ast Ast<'ast>, &'ast Ast<'ast>, &'ast Ast<'ast>, Span),
    Lambda(Vec<Symbol>, &'ast Ast<'ast>, Span),
    Define(Symbol, &'ast Ast<'ast>, Span),
    Block(Vec<&'ast Ast<'ast>>, Span),
}

pub fn parse<'ast>(s: &str,
                   interner: &mut SymbolIntern,
                   arena: &'ast Arena<Ast<'ast>>)
                   -> Result<Vec<&'ast Ast<'ast>>, errors::ParseError> {
    parse::parse(s, interner, arena)
}

impl<'ast> Ast<'ast> {
    pub fn dummy() -> Ast<'ast> {
        Ast::StringLit("dummy".into(), Span::dummy())
    }

    pub fn is_symbol_lit_with(&self, symbol: &Symbol) -> bool {
        if let &Ast::Symbol(ref s, _) = self {
            s == symbol
        } else {
            false
        }
    }

    pub fn span(&self) -> Span {
        use self::Ast::*;
        match *self {
            BoolLit(_, span) => span,
            StringLit(_, span) => span,
            IntLit(_, span) => span,
            FloatLit(_, span) => span,
            ListLit(_, span) => span,
            MapLit(_, span) => span,
            Symbol(_, span) => span,
            Add(_, span) => span,
            Quote(_, span) => span,
            List(_, span) => span,
            If(_, _, _, span) => span,
            Lambda(_, _, span) => span,
            Define(_, _, span) => span,
            Block( _, span) => span,
        }
    }

    pub fn equals_sans_span(&self, other: &Ast) -> bool {
        use self::Ast::*;
        match (self, other) {
            (&BoolLit(ref a, _), &BoolLit(ref b, _)) => a == b,
            (&StringLit(ref a, _), &StringLit(ref b, _)) => a == b,
            (&IntLit(ref a, _), &IntLit(ref b, _)) => a == b,
            (&FloatLit(ref a, _), &FloatLit(ref b, _)) => a == b,

            (&ListLit(ref a, _), &ListLit(ref b, _)) |
            (&List(ref a, _), &List(ref b, _)) |
            (&Add(ref a, _), &Add(ref b, _)) => {
                iterators_same(a.iter(), b.iter(), |&a, &b| Ast::equals_sans_span(a, b))
            }

            (&MapLit(ref a, _), &MapLit(ref b, _)) => {
                iterators_same(a.iter(), b.iter(), |&(k1, v1), &(k2, v2)| {
                    Ast::equals_sans_span(k1, k2) && Ast::equals_sans_span(v1, v2)
                })
            }
            (&Symbol(a, _), &Symbol(b, _)) => a == b,
            (&Quote(ref a, _), &Quote(ref b, _)) => a.equals_sans_span(&*b),
            (&If(ref ac, ref at, ref af, _),
             &If(ref bc, ref bt, ref bf, _)) => {
                ac.equals_sans_span(&*bc) && at.equals_sans_span(&*bt) && af.equals_sans_span(&*bf)
            }
            (&Lambda(ref a_args, ref a_bodies, _),
             &Lambda(ref b_args, ref b_bodies, _)) => {
                iterators_same(a_args.iter(), b_args.iter(), |a, b| a == b) &&
                a_bodies.equals_sans_span(b_bodies)
            }
            (&Define(ref s1, ref a1, _), &Define(ref s2, ref a2, _)) => {
                s1 == s2 && a1.equals_sans_span(a2)
            }
            (&Block(ref a_bodies, _),
             &Block(ref b_bodies, _)) => {
                iterators_same(a_bodies.iter(),
                               b_bodies.iter(),
                               |&a, &b| Ast::equals_sans_span(a, b))
            }

            _ => false,
        }
    }
}
