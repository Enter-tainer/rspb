use pulldown_cmark::{html, Options, Parser};
const CSS_TAG: &str = r#"<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/4.0.0/github-markdown.min.css" integrity="sha512-Oy18vBnbSJkXTndr2n6lDMO5NN31UljR8e/ICzVPrGpSud4Gkckb8yUpqhKuUNoE+o9gAb4O/rAxxw1ojyUVzg==" crossorigin="anonymous" />
<style>
	body {
		box-sizing: border-box;
		min-width: 200px;
		max-width: 980px;
		margin: 0 auto;
		padding: 45px;
	}

	@media (max-width: 767px) {
		body {
			padding: 15px;
		}
	}
</style>
"#;
pub fn render(md: String) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output.push_str(CSS_TAG);
    html_output
}
