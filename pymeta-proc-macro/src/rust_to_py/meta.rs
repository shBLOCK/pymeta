use crate::utils::parsing::SimpleRustPath;
use crate::utils::rust_token::{Ident, Punct};
use std::rc::Rc;

pub(crate) mod stmt {
    use super::*;
    use crate::rust_to_py::py_code_gen::PyCodeGen;
    use crate::utils::rust_token::TokenOptionEx;
    use crate::utils::rust_token::{Token, TokenBuffer};
    use proc_macro_error3::abort;
    use proc_macro2::Delimiter;

    #[derive(Debug)]
    pub struct MetaStmt {
        pub ident: Rc<Ident>,
        pub exclamation: Rc<Punct>,
        pub body: MetaStmtBody,
    }

    impl MetaStmt {
        pub fn parse(tokens: &mut TokenBuffer) -> Option<Self> {
            tokens.try_run_or_rewind(|tokens| {
                let ident = tokens.read_one().ident()?;
                let exclamation = tokens.read_one().expect_punct('!')?;
                let body = match ident.inner().to_string().as_str() {
                    "import" => MetaStmtBody::Import(ImportMetaStmt::parse(tokens)),
                    _ => return None,
                };
                Some(Self { ident, exclamation, body })
            })
        }
    }

    #[derive(Debug)]
    pub enum MetaStmtBody {
        Import(ImportMetaStmt),
        Cfg(), //TODO
    }

    impl MetaStmtBody {
        pub fn gen_py_code(&self, code_gen: &mut PyCodeGen) {
            todo!("good")
        }
    }

    #[derive(Debug)]
    pub struct ImportMetaStmt {
        pub path: Rc<SimpleRustPath>,
        pub items: ImportItems,
    }

    impl ImportMetaStmt {
        pub fn parse(tokens: &mut TokenBuffer) -> Self {
            let path = SimpleRustPath::parse(tokens).unwrap_or_else(|(span, txt)| abort!(span, txt));
            let items = match tokens.current() {
                Some(Token::Punct(dot)) if dot.eq_punct('.') => match tokens.seek(1).unwrap().current() {
                    Some(Token::Punct(star)) if star.eq_punct('*') => {
                        let star = star.clone();
                        tokens.seek(1).unwrap();
                        ImportItems::Wildcard { star: star.clone() }
                    }
                    Some(Token::Ident(_)) => ImportItems::Items(Box::new([ImportItem::parse(tokens)])),
                    Some(Token::Group(group)) => {
                        let group = group.clone();
                        tokens.seek(1).unwrap();
                        if group.delimiter() != Delimiter::Brace {
                            abort!(tokens.get_current_span_for_diagnostics(), "Expected braces");
                        }
                        let mut group_tokens = group.tokens();
                        let mut items = Vec::new();
                        let mut is_first = true;
                        while !group_tokens.exhausted() {
                            if !is_first {
                                if !tokens.read_one().eq_punct(',') {
                                    abort!(group_tokens.get_current_span_for_diagnostics(), "Expected `,`")
                                }
                            }
                            items.push(ImportItem::parse(&mut group_tokens));
                            is_first = false;
                        }
                        ImportItems::Items(items.into_boxed_slice())
                    }
                    _ => abort!(tokens.get_current_span_for_diagnostics(), "Unexpected token"),
                },
                _ => ImportItems::Module { r#as: ImportItemAs::parse(tokens) },
            };

            if !tokens.read_one().eq_punct(';') {
                abort!(tokens.get_current_span_for_diagnostics(), "Expected `;`");
            }

            Self { path: Rc::new(path), items }
        }
    }

    #[derive(Debug)]
    enum ImportItems {
        Module { r#as: Option<ImportItemAs> },
        Items(Box<[ImportItem]>),
        Wildcard { star: Rc<Punct> },
    }

    #[derive(Debug)]
    struct ImportItem {
        name: Rc<Ident>,
        r#as: Option<ImportItemAs>,
    }

    impl ImportItem {
        fn parse(tokens: &mut TokenBuffer) -> Self {
            let Some(name) = tokens.read_one().expect_ident_by(|s| s != "as") else {
                abort!(tokens.get_current_span_for_diagnostics(), "Expected identifier");
            };
            Self {
                name,
                r#as: ImportItemAs::parse(tokens),
            }
        }
    }

    #[derive(Debug)]
    struct ImportItemAs {
        kw: Rc<Ident>,
        name: Rc<Ident>,
    }

    impl ImportItemAs {
        fn parse(tokens: &mut TokenBuffer) -> Option<Self> {
            tokens.try_run_or_rewind(|tokens| {
                let kw = tokens.read_one().expect_ident("as")?;
                let Some(as_name) = tokens.read_one().expect_ident_by(|s| s != "as") else {
                    abort!(tokens.get_current_span_for_diagnostics(), "Expected identifier")
                };
                Some(ImportItemAs { kw, name: as_name })
            })
        }
    }
}

pub(crate) mod expr {
    use crate::utils::rust_token::TokenOptionEx;
    use crate::utils::rust_token::{Group, Ident, Punct, TokenBuffer};
    use proc_macro2::Delimiter;
    use std::rc::Rc;

    #[derive(Debug)]
    pub struct MetaExpr {
        pub ident: Rc<Ident>,
        pub exclamation: Rc<Punct>,
        pub group: Rc<Group>,
        pub body: MetaExprBody,
    }

    impl MetaExpr {
        pub fn parse(tokens: &mut TokenBuffer) -> Option<MetaExpr> {
            tokens.try_run_or_rewind(|tokens| {
                let ident = tokens.read_one().ident()?;
                let exclamation = tokens.read_one().expect_punct('!')?;
                let group = tokens.read_one().expect_group_by(|delim| delim != Delimiter::None)?;
                let body = MetaExprBody::parse(ident.inner().to_string().as_str(), &group)?;
                Some(Self { ident, exclamation, group, body })
            })
        }
    }

    #[derive(Debug)]
    pub enum MetaExprBody {
        Quote(QuoteMetaExpr),
        Pattern(PatternMetaExpr),
    }

    impl MetaExprBody {
        fn parse(ident: &str, group: &Rc<Group>) -> Option<MetaExprBody> {
            match ident {
                "quote" => todo!(),
                "pattern" => todo!(),
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    pub struct QuoteMetaExpr {}

    #[derive(Debug)]
    pub struct PatternMetaExpr {}
}
