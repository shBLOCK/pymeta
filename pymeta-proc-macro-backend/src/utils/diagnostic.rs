// #[cfg(proc_macro)]
// extern crate proc_macro;
// 
// use proc_macro2::Span;
// 
// #[derive(Copy, Clone, Debug)]
// pub enum DiagnosticLevel {
//     Error,
//     Warning,
//     Note,
//     Help,
// }
// 
// #[derive(Clone, Debug)]
// pub struct Diagnostic {
//     level: DiagnosticLevel,
//     message: String,
//     spans: Vec<Span>,
//     children: Vec<Diagnostic>,
// }
// impl Diagnostic {
//     pub fn new(spans: impl MultiSpan, level: DiagnosticLevel, message: impl Into<String>) -> Self {
//         Self {
//             spans: spans.into_spans(),
//             level,
//             message: message.into(),
//             children: Vec::new(),
//         }
//     }
// 
//     pub fn add_child(mut self, spans: impl MultiSpan, level: DiagnosticLevel, message: impl Into<String>) -> Self {
//         self.children.push(Self::new(spans, level, message));
//         self
//     }
// 
//     #[cfg(proc_macro)]
//     fn add_as_child_to(&self, mut diag: proc_macro::Diagnostic) -> proc_macro::Diagnostic {
//         if !self.spans.is_empty() {
//             let spans = self.spans.iter().map(|s| s.unwrap()).collect();
//             match self.level {
//                 DiagnosticLevel::Error => diag.span_error(spans, self.message),
//                 DiagnosticLevel::Warning => diag.span_warning(spans, self.message),
//                 DiagnosticLevel::Note => diag.span_note(spans, self.message),
//                 DiagnosticLevel::Help => diag.span_help(spans, self.message),
//             }
//         } else {
//             match self.level {
//                 DiagnosticLevel::Error => diag.error(self.message),
//                 DiagnosticLevel::Warning => diag.warning(self.message),
//                 DiagnosticLevel::Note => diag.note(self.message),
//                 DiagnosticLevel::Help => diag.help(self.message),
//             }
//         }
//     }
// 
//     #[cfg(proc_macro)]
//     pub fn emit(self) {
//         let mut diag = if !self.spans.is_empty() {
//             proc_macro::Diagnostic::spanned(
//                 self.spans.iter().map(|s| s.unwrap()).collect(),
//                 self.level,
//                 self.message,
//             )
//         } else {
//             proc_macro::Diagnostic::new(self.level, self.message)
//         };
//         for child in &self.children {
//             diag = child.add_as_child_to(diag);
//         }
//         diag.emit();
//     }
// 
//     #[cfg(not(proc_macro))]
//     pub fn emit(self) {
//         println!("{self:#?}")
//     }
// }
// 
// pub trait MultiSpan {
//     fn into_spans(self) -> Vec<Span>;
// }
// 
// impl MultiSpan for Span {
//     fn into_spans(self) -> Vec<Span> {
//         Vec::from([self])
//     }
// }
// 
// impl MultiSpan for Option<Span> {
//     fn into_spans(self) -> Vec<Span> {
//         if let Some(span) = self {
//             span.into_spans()
//         } else {
//             Vec::new()
//         }
//     }
// }
// 
// impl MultiSpan for &[Span] {
//     fn into_spans(self) -> Vec<Span> {
//         Vec::from(self)
//     }
// }
