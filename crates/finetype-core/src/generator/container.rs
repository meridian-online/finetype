//! Generators for the `container` domain.

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: container (12 types)
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn gen_container(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── object (6 types) ─────────────────────────────────────────
            ("object", "json") => {
                let templates = [
                    format!(
                        r#"{{"name":"{}","age":{},"active":{}}}"#,
                        self.random_first_name(),
                        self.rng.gen_range(18..80),
                        self.rng.gen_bool(0.7)
                    ),
                    format!(
                        r#"{{"id":{},"email":"{}@{}.com","role":"{}"}}"#,
                        self.rng.gen_range(1..10000),
                        self.random_first_name().to_lowercase(),
                        self.random_word(),
                        ["admin", "user", "moderator"][self.rng.gen_range(0..3)]
                    ),
                    format!(
                        r#"{{"product":"{}","price":{:.2},"currency":"{}"}}"#,
                        self.random_word(),
                        self.rng.gen::<f64>() * 999.0 + 0.01,
                        ["USD", "EUR", "GBP", "JPY"][self.rng.gen_range(0..4)]
                    ),
                    format!(
                        r#"{{"lat":{:.4},"lon":{:.4},"label":"{}"}}"#,
                        (self.rng.gen::<f64>() - 0.5) * 180.0,
                        (self.rng.gen::<f64>() - 0.5) * 360.0,
                        self.random_word()
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "json_array") => {
                let templates = [
                    format!(
                        "[{},{},{}]",
                        self.rng.gen_range(1..100),
                        self.rng.gen_range(1..100),
                        self.rng.gen_range(1..100)
                    ),
                    format!(
                        r#"["{}","{}","{}"]"#,
                        self.random_word(),
                        self.random_word(),
                        self.random_word()
                    ),
                    format!(
                        r#"[{{"id":{},"name":"{}"}},{{"id":{},"name":"{}"}}]"#,
                        self.rng.gen_range(1..100),
                        self.random_first_name(),
                        self.rng.gen_range(1..100),
                        self.random_first_name()
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "xml") => {
                let name = self.random_first_name();
                let age = self.rng.gen_range(18..80);
                let templates = [
                    format!("<person><name>{}</name><age>{}</age></person>", name, age),
                    format!(
                        "<item id=\"{}\"><title>{}</title><price>{:.2}</price></item>",
                        self.rng.gen_range(1..1000),
                        self.random_word(),
                        self.rng.gen::<f64>() * 100.0
                    ),
                    format!(
                        "<record><field name=\"status\">{}</field></record>",
                        ["active", "inactive", "pending"][self.rng.gen_range(0..3)]
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "html") => {
                let word1 = self.random_word();
                let word2 = self.random_word();
                let word3 = self.random_word();
                let name = self.random_first_name();
                let num = self.rng.gen_range(1..100);
                let templates = [
                    format!("<p>{} {} {}.</p>", word1, word2, word3),
                    format!(
                        "<div class=\"{}\"><a href=\"https://{}.com\">{}</a></div>",
                        word1, word2, word3
                    ),
                    format!("<h1>{}</h1><p>{} {} {}.</p>", name, word1, word2, word3),
                    format!(
                        "<ul><li>{}</li><li>{}</li><li>{}</li></ul>",
                        word1, word2, word3
                    ),
                    format!(
                        "<table><tr><td>{}</td><td>{}</td></tr></table>",
                        word1, num
                    ),
                    format!("<br><img src=\"{}.jpg\">", word1),
                    format!(
                        "<div id=\"main\"><h2>{}</h2><p>{} {}</p></div>",
                        name, word1, word2
                    ),
                    format!(
                        "<form action=\"/submit\"><input type=\"text\" name=\"{}\"><button>{}</button></form>",
                        word1, word2
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "yaml") => {
                let templates = [
                    format!(
                        "name: {}\nage: {}\nactive: {}",
                        self.random_first_name(),
                        self.rng.gen_range(18..80),
                        self.rng.gen_bool(0.7)
                    ),
                    format!(
                        "server:\n  host: {}.com\n  port: {}\n  ssl: true",
                        self.random_word(),
                        self.rng.gen_range(80..9000)
                    ),
                    format!(
                        "database:\n  driver: {}\n  name: {}",
                        ["postgres", "mysql", "sqlite"][self.rng.gen_range(0..3)],
                        self.random_word()
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "csv") => {
                let templates = [
                    format!(
                        "{},{},{},{}",
                        self.random_first_name(),
                        self.rng.gen_range(18..80),
                        self.random_first_name().to_lowercase() + "@example.com",
                        ["active", "inactive"][self.rng.gen_range(0..2)]
                    ),
                    format!(
                        "{},{:.2},{},{}",
                        self.random_word(),
                        self.rng.gen::<f64>() * 100.0,
                        self.rng.gen_range(1..1000),
                        ["USD", "EUR", "GBP"][self.rng.gen_range(0..3)]
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "s_expression") => {
                let (w1, w2, w3) = (self.random_word(), self.random_word(), self.random_word());
                let templates = [
                    format!("(ROOT (S (NP (NN {w1})) (VP (VBZ {w2}) (NP (NN {w3})))))"),
                    format!("(program (call (id {w1}) (string {w2})))"),
                    format!(
                        "(expr (op +) (num {}) (num {}))",
                        self.rng.gen_range(1..100),
                        self.rng.gen_range(1..100)
                    ),
                    format!("(node ({w1} {w2}) (child ({w3} leaf)))"),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }

            // ── array (4 types) ──────────────────────────────────────────
            ("array", "comma_separated") => {
                let count = self.rng.gen_range(3..8);
                if self.rng.gen_bool(0.5) {
                    // Words
                    let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                    Ok(items.join(","))
                } else {
                    // Numbers
                    let items: Vec<String> = (0..count)
                        .map(|_| self.rng.gen_range(1..100).to_string())
                        .collect();
                    Ok(items.join(","))
                }
            }
            ("array", "pipe_separated") => {
                let count = self.rng.gen_range(3..8);
                let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                Ok(items.join("|"))
            }
            ("array", "semicolon_separated") => {
                let count = self.rng.gen_range(3..8);
                let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                Ok(items.join(";"))
            }
            ("array", "whitespace_separated") => {
                let count = self.rng.gen_range(3..8);
                let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                if self.rng.gen_bool(0.7) {
                    Ok(items.join(" "))
                } else {
                    Ok(items.join("\t"))
                }
            }

            // ── key_value (2 types) ──────────────────────────────────────
            ("key_value", "query_string") => {
                let count = self.rng.gen_range(2..5);
                let pairs: Vec<String> = (0..count)
                    .map(|_| format!("{}={}", self.random_word(), self.random_word()))
                    .collect();
                Ok(pairs.join("&"))
            }
            _ => Err(GeneratorError::NotImplemented(format!(
                "container.{}.{}",
                category, type_name
            ))),
        }
    }
}
