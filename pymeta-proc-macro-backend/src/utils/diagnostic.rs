#[cfg(feature = "proc_macro")]
extern crate proc_macro;
use crate::utils::rust_token::Token;
use crate::utils::span::CSpan;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote_spanned};
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::panic::UnwindSafe;
use std::rc::Rc;
use std::sync::Mutex;
use std::thread::ThreadId;
use std::{panic, thread};

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

impl Display for DiagnosticLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
            Self::Help => "help",
        })
    }
}

#[cfg(all(feature = "proc_macro", feature = "nightly_diagnostic"))]
impl From<DiagnosticLevel> for proc_macro::Level {
    fn from(value: DiagnosticLevel) -> Self {
        match value {
            DiagnosticLevel::Error => proc_macro::Level::Error,
            DiagnosticLevel::Warning => proc_macro::Level::Warning,
            DiagnosticLevel::Note => proc_macro::Level::Note,
            DiagnosticLevel::Help => proc_macro::Level::Help,
        }
    }
}

macro_rules! diagnostic_add_child_methods {
    ($method_name:ident, $enum_name:ident) => {
        #[allow(unused)]
        pub fn $method_name(self, spans: impl MultiSpan, message: impl Into<String>) -> Self {
            self.add_child(spans, DiagnosticLevel::$enum_name, message)
        }
    };
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    level: DiagnosticLevel,
    message: String,
    spans: Vec<Span>,
    children: Vec<Diagnostic>,
}
impl Diagnostic {
    pub fn new(spans: impl MultiSpan, level: DiagnosticLevel, message: impl Into<String>) -> Self {
        Self {
            spans: spans.into_spans(),
            level,
            message: message.into(),
            children: Vec::new(),
        }
    }

    pub fn add_child(mut self, spans: impl MultiSpan, level: DiagnosticLevel, message: impl Into<String>) -> Self {
        self.children.push(Self::new(spans, level, message));
        self
    }

    diagnostic_add_child_methods!(add_error, Error);
    diagnostic_add_child_methods!(add_warning, Warning);
    diagnostic_add_child_methods!(add_note, Note);
    diagnostic_add_child_methods!(add_help, Help);

    #[cfg(all(feature = "proc_macro", feature = "nightly_diagnostic"))]
    fn add_as_child_to(&self, diag: proc_macro::Diagnostic) -> proc_macro::Diagnostic {
        let message = self.message.clone();
        if !self.spans.is_empty() {
            let spans: Vec<_> = self.spans.iter().map(|s| s.unwrap()).collect();
            match self.level {
                DiagnosticLevel::Error => diag.span_error(spans, message),
                DiagnosticLevel::Warning => diag.span_warning(spans, message),
                DiagnosticLevel::Note => diag.span_note(spans, message),
                DiagnosticLevel::Help => diag.span_help(spans, message),
            }
        } else {
            match self.level {
                DiagnosticLevel::Error => diag.error(message),
                DiagnosticLevel::Warning => diag.warning(message),
                DiagnosticLevel::Note => diag.note(message),
                DiagnosticLevel::Help => diag.help(message),
            }
        }
    }

    #[cfg(all(feature = "proc_macro", feature = "nightly_diagnostic"))]
    pub fn as_proc_macro_diagnostic(&self) -> proc_macro::Diagnostic {
        let mut diag = if !self.spans.is_empty() {
            proc_macro::Diagnostic::spanned(
                self.spans.iter().map(|s| s.unwrap()).collect::<Vec<_>>(),
                self.level.into(),
                self.message.clone(),
            )
        } else {
            proc_macro::Diagnostic::new(self.level.into(), self.message.clone())
        };
        for child in &self.children {
            diag = child.add_as_child_to(diag);
        }
        diag
    }

    pub fn emit(self) {
        #[cfg(not(feature = "proc_macro"))]
        eprintln!("{self:#?}");
        #[cfg(all(feature = "proc_macro", feature = "nightly_diagnostic"))]
        self.as_proc_macro_diagnostic().emit();
        get_context().as_mut().unwrap().diagnostics.push(self);
    }

    pub fn abort(self) -> ! {
        self.emit();
        crate::abort!()
    }
}

impl ToTokens for Diagnostic {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.level {
            DiagnosticLevel::Error => {}
            _ => return,
        }
        #[allow(clippy::useless_format)]
        let mut text = format!("{}", self.message);
        for child in &self.children {
            write!(text, "\n  = {}: {}", child.level, child.message).unwrap();
        }
        let spans = if !self.spans.is_empty() {
            self.spans.as_slice()
        } else {
            &[Span::call_site()]
        };
        for span in spans.iter().copied() {
            tokens.append_all(quote_spanned! {
                span => ::core::compile_error!(#text)
            });
        }
    }
}

pub trait MultiSpan {
    fn into_spans(self) -> Vec<Span>;
}

impl MultiSpan for Span {
    fn into_spans(self) -> Vec<Span> {
        Vec::from([self])
    }
}

impl MultiSpan for Option<Span> {
    fn into_spans(self) -> Vec<Span> {
        if let Some(span) = self {
            span.into_spans()
        } else {
            Vec::new()
        }
    }
}

impl MultiSpan for &Vec<Span> {
    fn into_spans(self) -> Vec<Span> {
        self.clone()
    }
}

impl MultiSpan for &[Span] {
    fn into_spans(self) -> Vec<Span> {
        Vec::from(self)
    }
}

impl MultiSpan for Rc<CSpan> {
    fn into_spans(self) -> Vec<Span> {
        self.inner().into_spans()
    }
}

impl MultiSpan for &Token {
    fn into_spans(self) -> Vec<Span> {
        self.span().into_spans()
    }
}

impl MultiSpan for &Option<&Token> {
    fn into_spans(self) -> Vec<Span> {
        self.map(|t| t.span().inner()).into_spans()
    }
}

static PROC_MACRO_THREAD: Mutex<Option<ThreadId>> = Mutex::new(None);
fn get_context() -> &'static mut Option<Context> {
    {
        match PROC_MACRO_THREAD.lock().unwrap().as_ref() {
            None => panic!("not in proc-macro context"),
            Some(&id) => {
                if thread::current().id() != id {
                    panic!("proc-macro context not in current thread");
                }
            }
        }
    }
    assert_eq!(Some(thread::current().id()), *PROC_MACRO_THREAD.lock().unwrap());
    static mut CONTEXT: Option<Context> = None;
    unsafe {
        #[allow(static_mut_refs)]
        &mut CONTEXT
    }
}

struct Context {
    diagnostics: Vec<Diagnostic>,
    dummy: Option<TokenStream>,
}

// TODO: this is not used for general `pymeta!` calls since it may hide the `compile_error!()` messages under stable
//       if the compiler decides to ignore the macro output because it's invalid in current context.
//       It's also not used for macro defining macros, but maybe we should output a dummy macro instead?
//       Expanding that dummy macro may lead to even more compile errors in some cases though (need to be tested).
#[allow(unused)]
pub fn set_dummy_output(tokens: TokenStream) {
    let _ = get_context().as_mut().unwrap().dummy.insert(tokens);
}

#[macro_export]
macro_rules! abort {
    () => {
        ::std::panic::panic_any($crate::utils::diagnostic::AbortPayload)
    };
    ($spans:expr, $msg:expr) => {
        $crate::utils::diagnostic::Diagnostic::new(
            $spans,
            $crate::utils::diagnostic::DiagnosticLevel::Error,
            $msg
        ).abort()
    };
    ($spans:expr, $msg:literal, $($args:expr),*) => {
        $crate::utils::diagnostic::Diagnostic::new(
            $spans,
            $crate::utils::diagnostic::DiagnosticLevel::Error,
            ::std::format!($msg, $($args),*)
        ).abort()
    };
}

#[doc(hidden)]
#[derive(Debug)]
pub struct AbortPayload;
impl Display for AbortPayload {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("AbortPayload")
    }
}

#[derive(Debug)]
pub struct ProcMacroResult {
    pub tokens: Option<TokenStream>,
    pub dummy: Option<TokenStream>,
    pub diagnostics: Box<[Diagnostic]>,
}
impl ProcMacroResult {
    pub fn resolve_to_tokens(self) -> TokenStream {
        #[allow(unused_mut)]
        let mut tokens = self.tokens.or(self.dummy).unwrap_or_else(TokenStream::new);
        #[cfg(not(feature = "nightly_diagnostic"))]
        {
            use proc_macro2::{Delimiter, TokenTree};
            use quote::quote;
            let diagnostics = self.diagnostics.iter();
            tokens = {
                let mut _tokens = tokens.clone().into_iter().collect::<Vec<_>>();
                if let [TokenTree::Group(group)] = _tokens.as_slice()
                    && group.delimiter() == Delimiter::Brace
                {
                    let group_tokens = group.stream();
                    quote! {
                        {
                            #(#diagnostics;)*
                            #group_tokens
                        }
                    }
                } else {
                    quote! {
                        #(#diagnostics;)*
                        #tokens
                    }
                }
            };
        }
        tokens
    }
}

pub fn run_proc_macro(f: impl (FnOnce() -> TokenStream) + UnwindSafe) -> ProcMacroResult {
    static PROC_MACRO_LOCK: Mutex<()> = Mutex::new(());
    let _proc_macro_lock = PROC_MACRO_LOCK.lock().unwrap();

    {
        let mut proc_macro_thread = PROC_MACRO_THREAD.lock().unwrap();
        assert!(proc_macro_thread.is_none(), "can't reenter run_proc_macro()");
        let _ = proc_macro_thread.insert(thread::current().id());
    }
    let _ = get_context().insert(Context { diagnostics: Vec::new(), dummy: None });
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if info.payload().is::<AbortPayload>() {
            // do nothing
        } else {
            default_hook(info);
        }
    }));
    let result = panic::catch_unwind(f);
    let _ = panic::take_hook();
    let context = { get_context().take().expect("Huh? Where did my Context go???") };
    PROC_MACRO_THREAD.lock().unwrap().take();

    let tokens = match result {
        Ok(it) => Some(it),
        Err(payload) => {
            if !payload.is::<AbortPayload>() {
                panic::resume_unwind(payload);
            }
            None
        }
    };
    ProcMacroResult {
        tokens,
        dummy: context.dummy,
        diagnostics: context.diagnostics.into(),
    }
}
