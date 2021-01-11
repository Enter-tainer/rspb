use lazy_static::lazy_static;
use syntect::parsing::SyntaxSet;
use syntect::{highlighting::ThemeSet};
#[derive(Debug)]
pub struct Highlighter {
    ps: syntect::parsing::SyntaxSet,
    ts: syntect::highlighting::ThemeSet,
}

pub fn highlight_lines(code: &String, ext: &String) -> String {
    lazy_static! {
        static ref HIGHLIGHTER: Highlighter = Highlighter {
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        };
    }
    let syntax = HIGHLIGHTER.ps.find_syntax_by_extension(ext).unwrap();
    syntect::html::highlighted_html_for_string(&code.as_str(), &HIGHLIGHTER.ps, syntax, &HIGHLIGHTER.ts.themes["InspiredGitHub"])
}
