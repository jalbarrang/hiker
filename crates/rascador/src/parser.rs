//! Parser — stage 2 of the compiler.
//!
//! This is a **recursive-descent** parser: there is roughly one function per
//! grammar rule, and the functions call each other the way the grammar nests.
//! Each function consumes some tokens and returns an AST node (or an error).
//!
//! Grammar (v0), informally:
//!   spec      := item*
//!   item      := sort | relation | law
//!   sort      := "sort" Ident ( "{" field ("," field)* "}" )?
//!   field     := Ident ":" Ident                 // type: "Int" or a sort name
//!   relation  := "relation" Ident "(" param ("," param)* ")"
//!   param     := Ident ":" Ident
//!   law       := "law" Ident "(" Ident ("," Ident)* ")" "{" clause* "}"
//!   clause    := pred ( "=>" pred )?
//!   pred      := expr op expr
//!   expr      := Int | Ident ( "." Ident )?
//!   op        := "==" | "<=" | "<" | ">=" | ">"

use crate::ast::*;
use crate::lexer::{tokenize, Tok, Token};

/// Convenience: lex + parse a source string into a `Spec`.
pub fn parse(src: &str) -> Result<Spec, String> {
    let tokens = tokenize(src)?;
    Parser::new(tokens).parse_spec()
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(toks: Vec<Token>) -> Self {
        Parser { toks, pos: 0 }
    }

    // ---- low-level token helpers -------------------------------------------

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    fn line(&self) -> usize {
        // Line of the current token, or the last token's line at EOF.
        self.toks
            .get(self.pos)
            .or_else(|| self.toks.last())
            .map_or(0, |t| t.line)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Consume a token that must equal `expected`, else error.
    fn expect(&mut self, expected: Tok) -> Result<(), String> {
        match self.peek() {
            Some(t) if *t == expected => {
                self.pos += 1;
                Ok(())
            }
            other => Err(format!(
                "line {}: expected {:?}, found {:?}",
                self.line(),
                expected,
                other
            )),
        }
    }

    /// Consume an identifier and return its text.
    fn expect_ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Ident(name)) => {
                let name = name.clone();
                self.pos += 1;
                Ok(name)
            }
            other => Err(format!(
                "line {}: expected a name, found {:?}",
                self.line(),
                other
            )),
        }
    }

    // ---- grammar rules ------------------------------------------------------

    fn parse_spec(&mut self) -> Result<Spec, String> {
        let mut spec = Spec {
            sorts: Vec::new(),
            relations: Vec::new(),
            laws: Vec::new(),
        };
        while let Some(tok) = self.peek() {
            match tok {
                Tok::Sort => spec.sorts.push(self.parse_sort()?),
                Tok::Relation => spec.relations.push(self.parse_relation()?),
                Tok::Law => spec.laws.push(self.parse_law()?),
                other => {
                    return Err(format!(
                        "line {}: expected `sort`, `relation`, or `law`, found {:?}",
                        self.line(),
                        other
                    ))
                }
            }
        }
        Ok(spec)
    }

    fn parse_sort(&mut self) -> Result<Sort, String> {
        let line = self.line();
        self.expect(Tok::Sort)?;
        let name = self.expect_ident()?;
        let mut fields = Vec::new();
        if self.peek() == Some(&Tok::LBrace) {
            self.expect(Tok::LBrace)?;
            // zero-or-more comma-separated fields until `}`
            while self.peek() != Some(&Tok::RBrace) {
                let field_name = self.expect_ident()?;
                self.expect(Tok::Colon)?;
                let ty_name = self.expect_ident()?;
                let ty = if ty_name == "Int" {
                    Ty::Int
                } else {
                    Ty::Sort(ty_name)
                };
                fields.push(Field {
                    name: field_name,
                    ty,
                });
                if self.peek() == Some(&Tok::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(Tok::RBrace)?;
        }
        Ok(Sort { name, fields, line })
    }

    fn parse_relation(&mut self) -> Result<Relation, String> {
        let line = self.line();
        self.expect(Tok::Relation)?;
        let name = self.expect_ident()?;
        self.expect(Tok::LParen)?;
        let mut params = Vec::new();
        while self.peek() != Some(&Tok::RParen) {
            let pname = self.expect_ident()?;
            self.expect(Tok::Colon)?;
            let psort = self.expect_ident()?;
            params.push(Param {
                name: pname,
                sort: psort,
            });
            if self.peek() == Some(&Tok::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(Tok::RParen)?;
        Ok(Relation { name, params, line })
    }

    fn parse_law(&mut self) -> Result<Law, String> {
        let line = self.line();
        self.expect(Tok::Law)?;
        let relation = self.expect_ident()?;
        self.expect(Tok::LParen)?;
        let mut args = Vec::new();
        while self.peek() != Some(&Tok::RParen) {
            args.push(self.expect_ident()?);
            if self.peek() == Some(&Tok::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(Tok::RParen)?;
        self.expect(Tok::LBrace)?;
        let mut clauses = Vec::new();
        while self.peek() != Some(&Tok::RBrace) {
            clauses.push(self.parse_clause()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(Law {
            relation,
            args,
            clauses,
            line,
        })
    }

    /// A clause is a comparison, optionally followed by `=> comparison`.
    fn parse_clause(&mut self) -> Result<Clause, String> {
        let line = self.line();
        let first = self.parse_pred()?;
        if self.peek() == Some(&Tok::FatArrow) {
            self.bump();
            let cons = self.parse_pred()?;
            Ok(Clause::Implies {
                ante: first,
                cons,
                line,
            })
        } else {
            Ok(Clause::Compare(first))
        }
    }

    fn parse_pred(&mut self) -> Result<Pred, String> {
        let line = self.line();
        let lhs = self.parse_expr()?;
        let op = self.parse_op()?;
        let rhs = self.parse_expr()?;
        Ok(Pred { lhs, op, rhs, line })
    }

    fn parse_op(&mut self) -> Result<CmpOp, String> {
        let op = match self.peek() {
            Some(Tok::EqEq) => CmpOp::Eq,
            Some(Tok::Le) => CmpOp::Le,
            Some(Tok::Lt) => CmpOp::Lt,
            Some(Tok::Ge) => CmpOp::Ge,
            Some(Tok::Gt) => CmpOp::Gt,
            other => {
                return Err(format!(
                    "line {}: expected a comparison operator, found {:?}",
                    self.line(),
                    other
                ))
            }
        };
        self.bump();
        Ok(op)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::Int(value)) => {
                self.bump();
                Ok(Expr::Int(value))
            }
            Some(Tok::Ident(name)) => {
                self.bump();
                // Optional `.field` makes it a field access.
                if self.peek() == Some(&Tok::Dot) {
                    self.bump();
                    let field = self.expect_ident()?;
                    Ok(Expr::Field { arg: name, field })
                } else {
                    Ok(Expr::Arg(name))
                }
            }
            other => Err(format!(
                "line {}: expected an expression, found {:?}",
                self.line(),
                other
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bare_sort() {
        let spec = parse("sort MediaItem").unwrap();
        assert_eq!(spec.sorts.len(), 1);
        assert_eq!(spec.sorts[0].name, "MediaItem");
        assert!(spec.sorts[0].fields.is_empty());
    }

    #[test]
    fn parses_sort_fields_with_types() {
        let spec = parse("sort P { media: MediaItem, t: Int }").unwrap();
        let f = &spec.sorts[0].fields;
        assert_eq!(f[0].name, "media");
        assert_eq!(f[0].ty, Ty::Sort("MediaItem".into()));
        assert_eq!(f[1].name, "t");
        assert_eq!(f[1].ty, Ty::Int);
    }

    #[test]
    fn parses_relation_and_law() {
        let src = "\
relation r(a: P, b: Q)
law r(a, b) {
  a.t <= b.t
  a.media == b.media
}";
        let spec = parse(src).unwrap();
        assert_eq!(spec.relations[0].params.len(), 2);
        let law = &spec.laws[0];
        assert_eq!(law.relation, "r");
        assert_eq!(law.args, vec!["a", "b"]);
        assert_eq!(law.clauses.len(), 2);
        let Clause::Compare(p0) = &law.clauses[0] else {
            panic!("expected a comparison clause");
        };
        assert_eq!(p0.op, CmpOp::Le);
        assert_eq!(
            p0.lhs,
            Expr::Field {
                arg: "a".into(),
                field: "t".into()
            }
        );
    }

    #[test]
    fn parses_an_implication_clause() {
        let src = "\
relation det(a: E, b: E)
law det(a, b) {
  a.tag == b.tag => a.status == b.status
}";
        let spec = parse(src).unwrap();
        let law = &spec.laws[0];
        assert_eq!(law.clauses.len(), 1);
        let Clause::Implies { ante, cons, .. } = &law.clauses[0] else {
            panic!("expected an implication clause");
        };
        assert_eq!(ante.op, CmpOp::Eq);
        assert_eq!(cons.op, CmpOp::Eq);
        assert_eq!(
            ante.lhs,
            Expr::Field {
                arg: "a".into(),
                field: "tag".into()
            }
        );
    }

    #[test]
    fn parses_the_real_spec() {
        let src = include_str!("../../../.rascador/temporal.tent");
        let spec = parse(src).unwrap();
        assert_eq!(spec.sorts.len(), 3);
        assert_eq!(spec.relations.len(), 2);
        assert_eq!(spec.laws.len(), 2);
    }

    #[test]
    fn reports_line_on_error() {
        let err = parse("sort X\nrelation r(p P)").unwrap_err(); // missing colon
        assert!(err.contains("line 2"), "got: {err}");
    }
}
