//! Lexer — stage 1 of the compiler.
//!
//! A lexer (a.k.a. tokenizer or scanner) turns a flat string of source text
//! into a list of **tokens** — the indivisible "words" of the language.
//! After this stage, nobody downstream has to care about whitespace, comments,
//! or how many spaces sat between two symbols.
//!
//! Example: the text `i.t0 <= p.t` becomes the tokens
//!   Ident("i") Dot Ident("t0") Le Ident("p") Dot Ident("t")
//!
//! We attach a line number to every token so later stages can produce error
//! messages that point at the offending line.

/// The kinds of token in the `.tent` language.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // Keywords (reserved identifiers).
    Sort,
    Relation,
    Law,
    Forbidden,

    // A name: sort names, relation names, field names, argument names.
    Ident(String),
    // An integer literal, e.g. `0`, `42`.
    Int(i64),

    // Punctuation.
    LBrace, // {
    RBrace, // }
    LParen, // (
    RParen, // )
    Comma,  // ,
    Dot,    // .
    Colon,  // :

    // Comparison operators used inside law predicates.
    EqEq, // ==
    Le,   // <=
    Lt,   // <
    Ge,   // >=
    Gt,   // >

    // Implication, separating a law clause's antecedent from its consequent.
    FatArrow, // =>
}

/// A token plus the source line it came from (1-indexed).
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
}

/// Turn source text into a vector of tokens.
///
/// Returns `Err(message)` on an unrecognized character, with the line number.
pub fn tokenize(src: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    // We scan over the bytes/chars with an index. `chars` lets us peek ahead
    // for two-character operators like `==` and `<=`.
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1;

    while i < chars.len() {
        let c = chars[i];

        // --- whitespace: skip, but count newlines so `line` stays accurate ---
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // --- line comments: `//` to end of line ---
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // --- two-character operators (check these before single-char) ---
        if c == '=' && i + 1 < chars.len() && chars[i + 1] == '=' {
            tokens.push(Token {
                tok: Tok::EqEq,
                line,
            });
            i += 2;
            continue;
        }
        if c == '=' && i + 1 < chars.len() && chars[i + 1] == '>' {
            tokens.push(Token {
                tok: Tok::FatArrow,
                line,
            });
            i += 2;
            continue;
        }
        if c == '<' && i + 1 < chars.len() && chars[i + 1] == '=' {
            tokens.push(Token { tok: Tok::Le, line });
            i += 2;
            continue;
        }
        if c == '>' && i + 1 < chars.len() && chars[i + 1] == '=' {
            tokens.push(Token { tok: Tok::Ge, line });
            i += 2;
            continue;
        }

        // --- single-character punctuation / operators ---
        let single = match c {
            '{' => Some(Tok::LBrace),
            '}' => Some(Tok::RBrace),
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            ',' => Some(Tok::Comma),
            '.' => Some(Tok::Dot),
            ':' => Some(Tok::Colon),
            '<' => Some(Tok::Lt),
            '>' => Some(Tok::Gt),
            _ => None,
        };
        if let Some(tok) = single {
            tokens.push(Token { tok, line });
            i += 1;
            continue;
        }

        // --- integer literals ---
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let value: i64 = text
                .parse()
                .map_err(|_| format!("line {line}: invalid integer `{text}`"))?;
            tokens.push(Token {
                tok: Tok::Int(value),
                line,
            });
            continue;
        }

        // --- identifiers and keywords ---
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let tok = match word.as_str() {
                "sort" => Tok::Sort,
                "relation" => Tok::Relation,
                "law" => Tok::Law,
                "forbidden" => Tok::Forbidden,
                _ => Tok::Ident(word),
            };
            tokens.push(Token { tok, line });
            continue;
        }

        return Err(format!("line {line}: unexpected character `{c}`"));
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Tok> {
        tokenize(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn tokenizes_a_predicate() {
        assert_eq!(
            kinds("i.t0 <= p.t"),
            vec![
                Tok::Ident("i".into()),
                Tok::Dot,
                Tok::Ident("t0".into()),
                Tok::Le,
                Tok::Ident("p".into()),
                Tok::Dot,
                Tok::Ident("t".into()),
            ]
        );
    }

    #[test]
    fn keywords_are_distinct_from_idents() {
        assert_eq!(
            kinds("sort relation law foo"),
            vec![Tok::Sort, Tok::Relation, Tok::Law, Tok::Ident("foo".into())]
        );
    }

    #[test]
    fn skips_comments_and_tracks_lines() {
        let toks = tokenize("// a comment\nsort X").unwrap();
        assert_eq!(toks[0].tok, Tok::Sort);
        assert_eq!(toks[0].line, 2); // sort is on line 2, after the comment
    }

    #[test]
    fn two_char_ops_beat_single_char() {
        assert_eq!(
            kinds("== <= >= < >"),
            vec![Tok::EqEq, Tok::Le, Tok::Ge, Tok::Lt, Tok::Gt]
        );
    }

    #[test]
    fn lexes_the_real_spec_without_error() {
        let src = include_str!("../../../.hiker/temporal.tent");
        assert!(tokenize(src).is_ok());
    }

    #[test]
    fn rejects_unknown_char() {
        let err = tokenize("sort X @").unwrap_err();
        assert!(err.contains("unexpected character"));
    }
}
