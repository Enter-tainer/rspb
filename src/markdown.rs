use pulldown_cmark::{html, Options, Parser};

pub fn render(md: String) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
