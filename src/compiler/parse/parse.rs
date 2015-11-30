use std::error::Error;
use vm::SymbolIntern;

use compiler::parse::Ast;
use compiler::parse::tokens::{TokenType, Token, Open, TokenIter};
pub use compiler::parse::errors::ParseError;
use compiler::parse::errors::ParseError::*;

fn one_expr<'a, 'b>(tok: Token,
                    tok_stream: &'a mut TokenIter<'b>,
                    interner: &mut SymbolIntern)
                    -> Result<Ast, ParseError> {
    use compiler::parse::tokens::TokenType;
    match tok.clone().tt {
        TokenType::Number(s) => {
            let as_int: Result<i64, _> = s.parse();
            if let Ok(int) = as_int {
                Ok(Ast::IntLit(int, tok.span))
            } else {
                let as_float: Result<f64, _> = s.parse();
                let as_float = as_float.map_err(|e| ConversionError(s, Box::new(e) as Box<Error>));
                as_float.map(|flt| Ast::FloatLit(flt, tok.span))
            }
        }
        TokenType::Symbol(s) => {
            match &s[..] {
                "true" => Ok(Ast::BoolLit(true, tok.span)),
                "false" => Ok(Ast::BoolLit(false, tok.span)),
                other => Ok(Ast::SymbolLit(interner.intern(other), tok.span))
            }
        }
        TokenType::String(s) => Ok(Ast::StringLit(s, tok.span)),
        // TODO: understand this
        TokenType::FormLike(_fl) => {
            unimplemented!();
            /* Ok({
            let quoted = try!(parse_one_expr(tok_stream, interner));
            let interned = Ast::Symbol(interner.intern(fl.form_name()));
            Ast::list(match quoted {
                None => vec![interned],
                Some(v) => vec![interned, v],
            })
            }), */
        }
        TokenType::Close(close) => Err(ExtraRightDelimiter(close, tok.span)),
        TokenType::Open(open) => {
            let (mut values, end_tok) = try!(parse_delimited(tok_stream, open, interner));
            match open {
                Open::LParen => {
                    if values.len() == 0 {
                        return Ok(Ast::List(values, tok.span.join(end_tok.span)))
                    }
                    if values[0].is_symbol_lit_with(&interner.precomputed.iff) {
                        let len = values.len();
                        let mut values = values.into_iter();

                        if len != 4 { return Err(UnexpectedIfArity(len, tok.span)); }
                        let _ = values.next();
                        let (cond, tru, fals) =
                            (values.next().unwrap(),
                             values.next().unwrap(),
                             values.next().unwrap());
                        Ok(Ast::If(
                            Box::new(cond),
                            Box::new(tru),
                            Box::new(fals),
                            tok.span.join(end_tok.span)))
                    } else if values[0].is_symbol_lit_with(&interner.precomputed.plus) {
                        values.remove(0);
                        Ok(Ast::Add(values, tok.span.join(end_tok.span)))
                    } else if values[0].is_symbol_lit_with(&interner.precomputed.lambda) {
                        if values.len() < 2 { return Err(UnexpectedLambdaArity(values.len(), tok.span)); }
                        values.remove(0); // remove the "lambda"
                        let args_list = values.remove(0);
                        let bodies = values;
                        if let Ast::List(args, t) = args_list {
                            let mut arg_list = vec![];
                            for arg in args {
                                if let Ast::SymbolLit(symbol, _) = arg {
                                    arg_list.push(symbol);
                                } else {
                                    return Err(BadLambdaArgs(t));
                                }
                            }
                            Ok(Ast::Lambda(arg_list, bodies, tok.span.join(end_tok.span)))
                        } else {
                            return Err(BadLambdaArgs(tok.span.join(end_tok.span)))
                        }
                    } else {
                        Ok(Ast::List(values, tok.span.join(end_tok.span)))
                    }
                },
                Open::LBracket => {
                    unimplemented!()
                    // TODO: add list literals back.
                    /*
                    if values.iter().all(|a| util::immediate_value(a, interner)) {
                        let values = values.into_iter().map(util::unquote).collect();
                        Ok(Ast::list(vec![Ast::Symbol(interner.intern("quote")),
                                            Ast::list(values)]))
                    } else {
                        values.insert(0, Ast::Symbol(interner.intern("list")));
                        Ok(Ast::list(values))
                    }*/
                }
                Open::LBrace => {
                    unimplemented!()
                    // TODO: add map literals back
                    /*
                    if values.len() % 2 == 1 {
                        return Err(InvalidMapLiteral(tok.start));
                    }
                    if values.iter().all(|a| util::immediate_value(a, interner)) {
                        let (keys, values): (Vec<_>, _) = values.into_iter()
                                                                .enumerate()
                                                                .partition(|&(i, _)| i % 2 == 0);
                        if keys.iter().all(|&(_, ref k)| util::can_be_hash_key(k, interner)) {
                            let m = keys.into_iter()
                                        .map(|(_, k)| util::unquote(k))
                                        .zip(values.into_iter().map(|(_, v)| util::unquote(v)))
                                        .collect();
                            Ok(Ast::Map(Rc::new(m)))
                        } else {
                            Err(InvalidMapLiteral(tok.start))
                        }
                    } else {
                        values.insert(0, Ast::Symbol(interner.intern("hash-map")));
                        Ok(Ast::list(values))
                    }
                    */
                }
            }
        }
    }
}

fn parse_one_expr<'a, 'b>(tok_stream: &'a mut TokenIter<'b>,
                          interner: &mut SymbolIntern)
                          -> Result<Option<Ast>, ParseError> {
    if let Some(tok) = tok_stream.next() {
        one_expr(try!(tok), tok_stream, interner).map(Some)
    } else {
        Ok(None)
    }
}

fn parse_delimited<'a, 'b>(tok_stream: &'a mut TokenIter<'b>,
                           opener: Open,
                           interner: &mut SymbolIntern)
                           -> Result<(Vec<Ast>, Token), ParseError> {
    let mut v = vec![];
    loop {
        if let Some(tok_or_err) = tok_stream.next() {
            let tok = try!(tok_or_err);
            match tok.tt {
                TokenType::Close(close) => if close == opener.closed_by() {
                    return Ok((v, tok));
                } else {
                    return Err(ExtraRightDelimiter(opener.closed_by(), tok.span));
                },
                _ => v.push(try!(one_expr(tok, tok_stream, interner))),
            }
        } else {
            return Err(MissingRightDelimiter(opener.closed_by()));
        }
    }
}

pub fn parse(input: &str, interner: &mut SymbolIntern) -> Result<Vec<Ast>, ParseError> {
    let mut v = vec![];
    let mut tok_iter = TokenIter::new(input);
    while let Some(value) = try!(parse_one_expr(&mut tok_iter, interner)) {
        v.push(value)
    }
    Ok(v)
}


#[cfg(test)]
mod tests {
    use compiler::parse::{Ast, Span};
    use vm::SymbolIntern;
    use super::parse;

    macro_rules! matches {
        ($e: expr, $p: pat) => {
            if let $p = $e { true } else { false }
        }
    }

    fn ok_parse(s: &str) -> (Vec<Ast>, SymbolIntern) {
        let mut interner = SymbolIntern::new();
        (parse(s, &mut interner).unwrap(), interner)
    }

    fn ok_parse_1(s: &str) -> (Ast, SymbolIntern) {
        let (mut parsed, interner) = ok_parse(s);
        assert!(parsed.len() == 1);
        (parsed.pop().unwrap(), interner)
    }

    #[test]
    fn test_parse_literals() {
        assert!(matches!(ok_parse_1("1").0, Ast::IntLit(1, _)));
        assert!(matches!(ok_parse_1("10.0").0, Ast::FloatLit(10.0, _)));
        assert!(matches!(ok_parse_1("true").0, Ast::BoolLit(true, _)));
        assert!(matches!(ok_parse_1("false").0, Ast::BoolLit(false, _)));
    }

    #[test]
    fn test_parse_if() {
        assert!(matches!(ok_parse_1("(if true 1 2)").0,
                         Ast::If(
                             box Ast::BoolLit(true, _),
                             box Ast::IntLit(1, _),
                             box Ast::IntLit(2, _),
                             _)));
        assert!(matches!(ok_parse_1("(if true (if false 1 3) 2)").0,
                         Ast::If(
                             box Ast::BoolLit(true, _),
                             box Ast::If(
                                 box Ast::BoolLit(false, _),
                                 box Ast::IntLit(1, _),
                                 box Ast::IntLit(3, _),
                                 _),
                             box Ast::IntLit(2, _),
                             _)));
    }

    #[test]
    fn test_parse_plus() {
        let ast = ok_parse_1("(+ 0 1 2)").0;

        let should = Ast::Add(vec![
            Ast::IntLit(0, Span::dummy()),
            Ast::IntLit(1, Span::dummy()),
            Ast::IntLit(2, Span::dummy())], Span::dummy());

        assert!(ast.equals_sans_span(&should));
    }

    #[test]
    fn test_parse_lambda() {
        let (ast, mut interner) = ok_parse_1("(lambda (a b c) (+ a b c))");
        let a = interner.intern("a");
        let b = interner.intern("b");
        let c = interner.intern("c");

        let should = Ast::Lambda(
            vec![a, b, c],
            vec![Ast::Add(vec![
                          Ast::SymbolLit(a, Span::dummy()),
                          Ast::SymbolLit(b, Span::dummy()),
                          Ast::SymbolLit(c, Span::dummy()),
                          ], Span::dummy())],
            Span::dummy());
        assert!(ast.equals_sans_span(&should));
    }
}
