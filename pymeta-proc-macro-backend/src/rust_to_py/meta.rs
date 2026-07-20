use crate::utils::parsing::RustSimplePath;
use crate::utils::rust_token::{Ident, Punct};
use std::rc::Rc;

pub(crate) mod stmt {
    use super::*;
    use crate::abort;
    use crate::rust_to_py::py_code_gen::PyCodeGen;
    use crate::rust_to_py::py_source::PySrcSegment;
    use crate::utils::rust_token::TokenOptionEx;
    use crate::utils::rust_token::{Token, TokenBuffer};
    use proc_macro2::Delimiter;

    #[derive(Debug)]
    pub struct MetaStmt {
        pub ident: Rc<Ident>,
        pub _exclamation: Rc<Punct>,
        pub body: MetaStmtBody,
    }

    impl MetaStmt {
        pub fn try_parse(tokens: &mut TokenBuffer) -> Option<Self> {
            tokens.try_run_or_rewind(|tokens| {
                let ident = tokens.read_one().ident().ok()?;
                let exclamation = tokens.read_one().expect_punct('!').ok()?;
                let body = match ident.inner().to_string().as_str() {
                    "import" => MetaStmtBody::Import(ImportMetaStmt::parse(tokens)),
                    _ => return None,
                };
                Some(Self {
                    ident,
                    _exclamation: exclamation,
                    body,
                })
            })
        }

        pub fn codegen(&self, pcg: &mut PyCodeGen) {
            match &self.body {
                MetaStmtBody::Import(body) => body.codegen(self, pcg),
            }
        }
    }

    #[derive(Debug)]
    pub enum MetaStmtBody {
        Import(ImportMetaStmt),
    }

    #[derive(Debug)]
    pub struct ImportMetaStmt {
        pub path: Rc<RustSimplePath>,
        pub(self) items: ImportItems,
        semicolon: Rc<Punct>,
    }

    impl ImportMetaStmt {
        pub fn parse(tokens: &mut TokenBuffer) -> Self {
            let path = RustSimplePath::try_parse(tokens).unwrap_or_else(|e| e.abort());
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
                                if !group_tokens.read_one().eq_punct(',') {
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

            let Ok(semicolon) = tokens.read_one().expect_punct(';') else {
                abort!(tokens.get_current_span_for_diagnostics(), "Expected `;`");
            };

            Self {
                path: Rc::new(path),
                items,
                semicolon,
            }
        }

        pub const PATH: &str = "pymeta._modules";
        pub fn module_name(path: &Rc<RustSimplePath>) -> String {
            let mut name = String::new();
            for (i, seg) in path.segments.iter().enumerate() {
                if i != 0 || path.is_root() {
                    name.push_str("__");
                }
                name.push_str(seg.inner().to_string().as_str());
            }
            name
        }

        pub fn codegen(&self, meta_stmt: &MetaStmt, pcg: &mut PyCodeGen) {
            let module_name = PySrcSegment::new(
                Self::module_name(&self.path),
                Some(Rc::new(self.path.total_span().into())),
            );

            pcg.py.new_line(None);
            match &self.items {
                ImportItems::Module { r#as } => {
                    pcg.py.append(("import", meta_stmt.ident.span()));
                    pcg.py.append(" ");
                    pcg.py.append(Self::PATH);
                    pcg.py.append(".");
                    pcg.py.append(module_name);
                    if let Some(r#as) = r#as {
                        r#as.codegen(pcg);
                    } else {
                        pcg.py.append(" as ");
                        pcg.py.append(self.path.segments.last().unwrap());
                    }
                }
                ImportItems::Items(_) | ImportItems::Wildcard { .. } => {
                    pcg.py.append(("from", meta_stmt.ident.span()));
                    pcg.py.append(" ");
                    pcg.py.append(Self::PATH);
                    pcg.py.append(".");
                    pcg.py.append(module_name);
                    pcg.py.append(" ");
                    pcg.py.append(("import", meta_stmt.ident.span()));
                    pcg.py.append(" ");
                    match &self.items {
                        ImportItems::Items(items) => {
                            pcg.py.append("(");
                            for (i, item) in items.iter().enumerate() {
                                if i != 0 {
                                    pcg.py.append(", ");
                                }
                                item.codegen(pcg);
                            }
                            pcg.py.append(")");
                        }
                        ImportItems::Wildcard { star } => {
                            pcg.py.append(star);
                        }
                        ImportItems::Module { .. } => unreachable!(),
                    }
                }
            }
            pcg.py.append(&self.semicolon);
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
            let Ok(name) = tokens.read_one().expect_ident_by(|s| s != "as") else {
                abort!(tokens.get_current_span_for_diagnostics(), "Expected identifier");
            };
            Self {
                name,
                r#as: ImportItemAs::parse(tokens),
            }
        }

        fn codegen(&self, pcg: &mut PyCodeGen) {
            pcg.py.append(&self.name);
            if let Some(ref r#as) = self.r#as {
                r#as.codegen(pcg);
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
                let kw = tokens.read_one().expect_ident("as").ok()?;
                let Ok(name) = tokens.read_one().expect_ident_by(|s| s != "as") else {
                    abort!(tokens.get_current_span_for_diagnostics(), "Expected identifier")
                };
                Some(ImportItemAs { kw, name })
            })
        }

        fn codegen(&self, pcg: &mut PyCodeGen) {
            pcg.py.append(" ");
            pcg.py.append(&self.kw);
            pcg.py.append(" ");
            pcg.py.append(&self.name);
        }
    }
}

#[allow(unused)] // tmp
pub(crate) mod expr {
    use crate::abort;
    use crate::utils::rust_token::TokenOptionEx;
    use crate::utils::rust_token::{Group, Ident, Punct, TokenBuffer};
    use proc_macro2::Delimiter;
    use std::rc::Rc;

    #[derive(Debug)]
    pub struct MetaExpr {
        pub _ident: Rc<Ident>,
        pub _exclamation: Rc<Punct>,
        pub _group: Rc<Group>,
        pub _body: MetaExprBody,
    }

    impl MetaExpr {
        pub fn parse(tokens: &mut TokenBuffer) -> Option<MetaExpr> {
            tokens.try_run_or_rewind(|tokens| {
                let ident = tokens.read_one().ident().ok()?;
                let exclamation = tokens.read_one().expect_punct('!').ok()?;
                let group = tokens
                    .read_one()
                    .expect_group_by(|delim| delim != Delimiter::None)
                    .ok()?;
                let body = MetaExprBody::parse(&ident, &group);
                Some(Self {
                    _ident: ident,
                    _exclamation: exclamation,
                    _group: group,
                    _body: body,
                })
            })
        }
    }

    #[derive(Debug)]
    pub enum MetaExprBody {
        // Pattern(PatternMetaExpr),
    }

    impl MetaExprBody {
        fn parse(ident: &Rc<Ident>, group: &Rc<Group>) -> MetaExprBody {
            let _ = group;
            abort!(ident.span(), "Meta expressions are not yet implemented");
            // match ident.inner().to_string().as_str() {
            //     "pattern" => ,
            //     ident => abort!(ident.span(), "Unknown meta expression: {}", ident),
            // }
        }
    }

    // #[derive(Debug)]
    // pub struct QuoteMetaExpr {}
}
