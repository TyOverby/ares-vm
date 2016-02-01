use typed_arena::Arena;
use std::collections::HashMap;

mod error;
pub use self::error::BindingError;

use compiler::parse::Ast;
use ares_syntax::{Symbol, SymbolIntern};
use util::iterators_same;

// 2 concepts, Binders and Bound nodes
//
// Binders are for things that introduce bindings
//
// Bounds are for every node that needs binding

// Each bound node has a reference to the AST that it was from,
//

#[derive(Debug)]
pub enum Bound<'bound, 'ast: 'bound> {
    Literal(&'ast Ast<'ast>),
    Symbol {
        symbol: Symbol,
        ast: &'ast Ast<'ast>,
        source: SymbolBindSource,
    },

    ListLit(Vec<&'bound Bound<'bound, 'ast>>, &'ast Ast<'ast>),
    MapLit(Vec<(&'bound Bound<'bound, 'ast>, &'bound Bound<'bound, 'ast>)>, &'ast Ast<'ast>),
    Add(Vec<&'bound Bound<'bound, 'ast>>, &'ast Ast<'ast>),
    Quote {
        quoting: &'ast Ast<'ast>,
        ast: &'ast Ast<'ast>,
    },
    List(Vec<&'bound Bound<'bound, 'ast>>, &'ast Ast<'ast>),
    If(&'bound Bound<'bound, 'ast>,
       &'bound Bound<'bound, 'ast>,
       &'bound Bound<'bound, 'ast>,
       &'ast Ast<'ast>),
    Lambda {
        arg_symbols: Vec<Symbol>,
        body: &'bound Bound<'bound, 'ast>,
        ast: &'ast Ast<'ast>,
        bindings: LambdaBindings,
    },
    Block(Vec<&'bound Bound<'bound, 'ast>>, &'ast Ast<'ast>),
    Define(Symbol, SymbolBindSource, &'bound Bound<'bound, 'ast>, &'ast Ast<'ast>),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SymbolBindSource {
    Arg(u32),
    Upvar(u32),
    LocalDefine(u32),
    Global(Symbol),
}

struct BuckStopsHereBinder;

#[derive(Debug, Eq, PartialEq)]
pub struct LambdaBindings {
    pub bindings: HashMap<Symbol, SymbolBindSource>,
    pub num_args: u32,
    pub num_upvars: u32,
    pub num_declarations: u32,
}

struct LambdaBinder<'a> {
    parent: &'a mut Binder,
    args: &'a Vec<Symbol>,
    bindings: LambdaBindings,
}

struct BlockBinder<'a> {
    parent: &'a mut Binder,
    symbol_map: HashMap<Symbol, Symbol>
}

trait Binder {
    fn add_declaration(&mut self, symbol: Symbol, interner: &mut SymbolIntern) -> SymbolBindSource;
    fn already_binds(&self, symbol: Symbol) -> bool;
    fn lookup(&self, symbol: Symbol) -> Option<SymbolBindSource>;
}

impl LambdaBindings {
    fn new() -> LambdaBindings {
        LambdaBindings {
            bindings: HashMap::new(),
            num_args: 0,
            num_upvars: 0,
            num_declarations: 0,
        }
    }

    pub fn compute_stack_offset(&self, bind_source: SymbolBindSource) -> u32 {
        match bind_source {
            SymbolBindSource::Arg(a) => a,
            SymbolBindSource::Upvar(_) => unimplemented!(),
            SymbolBindSource::LocalDefine(a) => self.num_args + self.num_upvars + a,
            SymbolBindSource::Global(_) => unimplemented!(),
        }
    }
}

impl<'a> LambdaBinder<'a> {
    fn new(parent: &'a mut Binder, args: &'a Vec<Symbol>) -> LambdaBinder<'a> {
        let mut bindings = LambdaBindings::new();
        for (i, arg_symbol) in args.iter().enumerate() {
            bindings.bindings.insert(arg_symbol.clone(), SymbolBindSource::Arg(i as u32));
        }
        bindings.num_args = args.len() as u32;

        LambdaBinder {
            parent: parent,
            args: args,
            bindings: bindings,
        }
    }
}

impl<'a> Binder for LambdaBinder<'a> {
    fn add_declaration(&mut self,
                       symbol: Symbol,
                       _interner: &mut SymbolIntern)
                       -> SymbolBindSource {
        assert!(!self.bindings.bindings.contains_key(&symbol));
        let source = SymbolBindSource::LocalDefine(self.bindings.num_declarations);
        self.bindings.bindings.insert(symbol, source);
        self.bindings.num_declarations += 1;
        source
    }

    fn already_binds(&self, symbol: Symbol) -> bool {
        self.bindings.bindings.contains_key(&symbol)
    }

    fn lookup(&self, symbol: Symbol) -> Option<SymbolBindSource> {
        self.bindings.bindings.get(&symbol).cloned()
    }
}

impl <'a> BlockBinder<'a> {
    fn new(parent: &'a mut Binder) -> BlockBinder<'a> {
        BlockBinder {
            parent: parent,
            symbol_map: HashMap::new(),
        }
    }
}

impl <'a> Binder for BlockBinder<'a> {
    fn add_declaration(&mut self, symbol: Symbol, interner: &mut SymbolIntern) -> SymbolBindSource {
        let mask = interner.gensym();
        self.symbol_map.insert(symbol, mask);
        self.parent.add_declaration(mask, interner)
    }

    fn already_binds(&self, symbol: Symbol) -> bool {
        self.symbol_map.contains_key(&symbol) || self.parent.already_binds(symbol)
    }

    fn lookup(&self, symbol: Symbol) -> Option<SymbolBindSource> {
        match self.symbol_map.get(&symbol) {
            Some(&translated) => self.parent.lookup(translated),
            None => self.parent.lookup(symbol)
        }
    }
}

impl Binder for BuckStopsHereBinder {
    fn add_declaration(&mut self,
                       symbol: Symbol,
                       _interner: &mut SymbolIntern)
                       -> SymbolBindSource {
        SymbolBindSource::Global(symbol)
    }

    fn already_binds(&self, _symbol: Symbol) -> bool {
        false
    }

    fn lookup(&self, symbol: Symbol) -> Option<SymbolBindSource> {
        Some(SymbolBindSource::Global(symbol))
    }
}

impl<'bound, 'ast: 'bound> Bound<'bound, 'ast> {
    pub fn bind_top(ast: &'ast Ast<'ast>,
                    arena: &'bound Arena<Bound<'bound, 'ast>>,
                    interner: &mut SymbolIntern)
                    -> Result<&'bound Bound<'bound, 'ast>, BindingError> {
        let mut buck = BuckStopsHereBinder;
        Bound::bind(ast, arena, &mut buck, interner)
    }

    fn bind(ast: &'ast Ast<'ast>,
            arena: &'bound Arena<Bound<'bound, 'ast>>,
            binder: &mut Binder,
            interner: &mut SymbolIntern)
            -> Result<&'bound Bound<'bound, 'ast>, BindingError> {
        Ok(arena.alloc(match ast {
            &Ast::BoolLit(_, _) |
            &Ast::StringLit(_, _) |
            &Ast::FloatLit(_, _) |
            &Ast::IntLit(_, _) => Bound::Literal(ast),
            &Ast::ListLit(ref elements, _) => {
                Bound::ListLit(try!(elements.iter()
                                            .map(|element| {
                                                Bound::bind(element, arena, binder, interner)
                                            })
                                            .collect::<Result<Vec<_>, _>>()),
                               ast)
            }
            &Ast::MapLit(ref elements, _) => {
                let mut bound = Vec::with_capacity(elements.len());
                for &(ref k, ref v) in elements {
                    let k = try!(Bound::bind(k, arena, binder, interner));
                    let v = try!(Bound::bind(v, arena, binder, interner));
                    bound.push((k, v));
                }
                Bound::MapLit(bound, ast)
            }
            &Ast::Symbol(symbol, span) => {
                let source = match binder.lookup(symbol) {
                    Some(source) => source,
                    None => return Err(BindingError::CouldNotBind(symbol, span)),
                };

                Bound::Symbol {
                    symbol: symbol,
                    ast: ast,
                    source: source,
                }
            }
            &Ast::Add(ref elements, _) => {
                let mut bound = Vec::with_capacity(elements.len());
                for element in elements {
                    bound.push(try!(Bound::bind(element, arena, binder, interner)));
                }
                Bound::Add(bound, ast)
            }
            &Ast::Quote(ref q, _) => {
                Bound::Quote {
                    quoting: q,
                    ast: ast,
                }
            }
            &Ast::List(ref elements, _) => {
                let mut bound = Vec::with_capacity(elements.len());
                for element in elements {
                    bound.push(try!(Bound::bind(element, arena, binder, interner)));
                }
                Bound::List(bound, ast)
            }
            &Ast::If(ref a, ref b, ref c, _) => {
                Bound::If(try!(Bound::bind(a, arena, binder, interner)) as &_,
                          try!(Bound::bind(b, arena, binder, interner)) as &_,
                          try!(Bound::bind(c, arena, binder, interner)) as &_,
                          ast)
            }
            &Ast::Lambda(ref args, ref body_block, _) => {
                let mut new_binder = LambdaBinder::new(binder, args);
                let bound_body = try!(Bound::bind(body_block, arena, &mut new_binder, interner));
                Bound::Lambda {
                    arg_symbols: args.clone(),
                    body: bound_body,
                    ast: ast,
                    bindings: new_binder.bindings,
                }
            }
            &Ast::Block(ref bodies, _) => {
                let mut new_binder = BlockBinder::new(binder);
                let mut bound_bodies = Vec::with_capacity(bodies.len());
                for body in bodies {
                    bound_bodies.push(try!(Bound::bind(body, arena, &mut new_binder, interner)));
                }
                Bound::Block(bound_bodies, ast)
            }
            &Ast::Define(symbol, value, _) => {
                if binder.already_binds(symbol) {
                    return Err(BindingError::AlreadyDefined(symbol));
                }
                let source = binder.add_declaration(symbol, interner);
                let bound_value = try!(Bound::bind(value, arena, binder, interner));
                Bound::Define(symbol, source, bound_value, ast)
            }
        }))
    }

    fn equals_sans_ast(&self, other: &'bound Bound<'bound, 'ast>) -> bool {
        match (self, other) {
            (&Bound::Literal(ref a), &Bound::Literal(ref b)) => {
                let res = a.equals_sans_span(b);
                res
            }

            (&Bound::Symbol {
                symbol: symbol_a,
                source: source_a,
                ..
            },
             &Bound::Symbol {
                symbol: symbol_b,
                source: source_b,
                ..
            }) => symbol_a == symbol_b && source_a == source_b,

            (&Bound::ListLit(ref list_a, _),
             &Bound::ListLit(ref list_b, _)) |
            (&Bound::List(ref list_a, _), &Bound::List(ref list_b, _)) |
            (&Bound::Add(ref list_a, _), &Bound::Add(ref list_b, _)) => {
                iterators_same(list_a.iter(),
                               list_b.iter(),
                               |&a, &b| Bound::equals_sans_ast(a, b))
            }

            (&Bound::MapLit(ref list_a, _), &Bound::MapLit(ref list_b, _)) => {
                iterators_same(list_a.iter(), list_b.iter(), |&(k1, v1), &(k2, v2)| {
                    Bound::equals_sans_ast(k1, k2) && Bound::equals_sans_ast(v1, v2)
                })
            }
            (&Bound::Quote { quoting: quoting_a, ..  },
             &Bound::Quote { quoting: quoting_b, ..  }) => quoting_a.equals_sans_span(quoting_b),
            (&Bound::If(ref a1, ref a2, ref a3, _),
             &Bound::If(ref b1, ref b2, ref b3, _)) => {
                a1.equals_sans_ast(b1) && a2.equals_sans_ast(b2) && a3.equals_sans_ast(b3)
            }
            (&Bound::Lambda{arg_symbols: ref args_a, body: ref body_a, bindings: ref bindings_a, ast: ref _asta },
             &Bound::Lambda{arg_symbols: ref args_b, body: ref body_b, bindings: ref bindings_b, ast: ref _astb }) => {
                let mut res = iterators_same(args_a.iter(), args_b.iter(), |a, b| a == b);
                res &= body_a.equals_sans_ast(body_b);
                res &= bindings_a == bindings_b;
                res
            }
            (&Bound::Define(symbol_a, source_a, bound_a, _),
             &Bound::Define(symbol_b, source_b, bound_b, _)) => {
                let defines_eq = symbol_a == symbol_b && source_a == source_b &&
                                 bound_a.equals_sans_ast(bound_b);
                defines_eq
            }
            (&Bound::Block(ref bodies_a, _),
             &Bound::Block(ref bodies_b, _)) => {
                iterators_same(bodies_a.iter(), bodies_b.iter(), |&a, &b| Bound::equals_sans_ast(a, b))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Bound, SymbolBindSource, LambdaBindings};
    use compiler::parse::Ast;
    use compiler::parse::test::ok_parse_1;
    use typed_arena::Arena;

    #[test]
    fn bind_lambda_one_arg() {
        let parse_arena = Arena::new();
        let bind_arena = Arena::new();
        let (ast, mut interner) = ok_parse_1("(lambda (a) a)", &parse_arena);
        let bound = Bound::bind_top(ast, &bind_arena, &mut interner);

        let should = bind_arena.alloc(Bound::Lambda {
            arg_symbols: vec![interner.intern("a")],
            body: bind_arena.alloc(Bound::Block(vec![bind_arena.alloc(Bound::Symbol {
                                   symbol: interner.intern("a"),
                                   ast: parse_arena.alloc(Ast::dummy()),
                                   source: SymbolBindSource::Arg(0),
                               })], parse_arena.alloc(Ast::dummy()))),
            ast: parse_arena.alloc(Ast::dummy()),
            bindings: LambdaBindings {
                bindings: vec![(interner.intern("a"), SymbolBindSource::Arg(0))]
                              .into_iter()
                              .collect(),
                num_args: 1,
                num_upvars: 0,
                num_declarations: 0,
            },
        });
        assert!(should.equals_sans_ast(bound.unwrap()));
    }

    #[test]
    fn bind_lambda_two_args() {
        let parse_arena = Arena::new();
        let bind_arena = Arena::new();
        let (ast, mut interner) = ok_parse_1("(lambda (a b) (+ a b))", &parse_arena);
        let bound = Bound::bind_top(ast, &bind_arena, &mut interner);

        let should = bind_arena.alloc(Bound::Lambda {
            arg_symbols: vec![interner.intern("a"), interner.intern("b")],
            body: bind_arena.alloc(Bound::Block(vec![bind_arena.alloc(Bound::Add(vec![
                                  bind_arena.alloc(Bound::Symbol {
                                      symbol: interner.intern("a"),
                                      ast: parse_arena.alloc(Ast::dummy()),
                                      source: SymbolBindSource::Arg(0)
                                  }),
                                  bind_arena.alloc(Bound::Symbol {
                                      symbol: interner.intern("b"),
                                      ast: parse_arena.alloc(Ast::dummy()),
                                      source: SymbolBindSource::Arg(1)
                                  })
                           ],
                                                           parse_arena.alloc(Ast::dummy())))], parse_arena.alloc(Ast::dummy()))),
            ast: parse_arena.alloc(Ast::dummy()),
            bindings: LambdaBindings {
                bindings: vec![(interner.intern("a"), SymbolBindSource::Arg(0)),
                               (interner.intern("b"), SymbolBindSource::Arg(1))]
                              .into_iter()
                              .collect(),
                num_args: 2,
                num_upvars: 0,
                num_declarations: 0,
            },
        });
        assert!(should.equals_sans_ast(bound.unwrap()));
    }

    /*
    #[test]
    fn bind_lambda_with_define() {
        let parse_arena = Arena::new();
        let bind_arena = Arena::new();
        let (ast, mut interner) = ok_parse_1("(lambda () (define x 5) x)", &parse_arena);
        let bound = Bound::bind_top(ast, &bind_arena, &mut interner);
        let x = interner.intern("x");

        let should = bind_arena.alloc(Bound::Lambda {
            arg_symbols: vec![],
            body: bind_arena.alloc(Bound::Block(vec![
                    bind_arena.alloc(
                        Bound::Define(x, SymbolBindSource::LocalDefine(0), bind_arena.alloc(
                                Bound::Literal(
                                    parse_arena.alloc(Ast::IntLit(5, Span::dummy())))
                                ), parse_arena.alloc(Ast::dummy()))),
                    bind_arena.alloc(
                        Bound::Symbol {
                            symbol: x,
                            ast: parse_arena.alloc(Ast::dummy()),
                            source: SymbolBindSource::LocalDefine(0)
                        })
                ], parse_arena.alloc(Ast::dummy()))),
            ast: parse_arena.alloc(Ast::dummy()),
            bindings: LambdaBindings {
                bindings: vec![(x, SymbolBindSource::LocalDefine(0))].into_iter().collect(),
                num_args: 0,
                num_upvars: 0,
                num_declarations: 1,
            },
        });

        println!("parsed {:#?}", bound);
        println!("manual {:#?}", should);

        assert!(should.equals_sans_ast(bound.unwrap()));
    }
    */
}
