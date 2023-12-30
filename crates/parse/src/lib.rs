//! Solidity parser.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/danipopes/sulk/main/assets/logo.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/danipopes/sulk/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use sulk_interface::diagnostics::{DiagnosticBuilder, ErrorGuaranteed};

pub mod lexer;
pub use lexer::Lexer;

mod parser;
pub use parser::Parser;

mod session;
pub use session::ParseSess;

/// Parser error type.
pub type PErr<'a> = DiagnosticBuilder<'a, ErrorGuaranteed>;

/// Parser result type. This is a shorthand for `Result<T, PErr<'a>>`.
pub type PResult<'a, T> = Result<T, PErr<'a>>;