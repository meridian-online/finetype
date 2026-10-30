//! The `display` block on a taxonomy definition: how a type's values are shown.
//!
//! A block carries either a d3-format specifier (`number`) or a d3-time-format
//! specifier (`time`), plus flags for facts a specifier cannot express. It
//! holds only what the label implies; scale and precision stay with the caller.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

fn is_false(b: &bool) -> bool {
    !*b
}

/// How a definition's values are displayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Display {
    /// d3-format specifier for the stored number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    /// d3-time-format specifier for the stored date, time or timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// The value is an amount of money.
    #[serde(default, skip_serializing_if = "is_false")]
    pub money: bool,
    /// ISO 4217 code, only when the label itself fixes one currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// The stored value is a fraction, shown as a percentage.
    #[serde(default, skip_serializing_if = "is_false")]
    pub fraction: bool,
    /// The value is a size in bytes.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bytes: bool,
}

fn number_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(?:(?s:.)?[<>=^])?[-+( ]?#?0?\d*,?(?:\.\d+)?~?([efgrs%pboxXcd]?)$")
            .expect("number specifier regex")
    })
}

const TIME_DIRECTIVES: &str = "aAbBcdefgGHIjLmMpqQsSuUVwWxXyYZ%";

fn validate_time(spec: &str) -> Result<(), String> {
    let mut chars = spec.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            continue;
        }
        let mut next = chars
            .next()
            .ok_or_else(|| format!("time specifier {spec:?} ends in a lone '%'"))?;
        if matches!(next, '0' | '_' | '-') {
            next = chars.next().ok_or_else(|| {
                format!("time specifier {spec:?} ends in a modifier with no directive")
            })?;
        }
        if !TIME_DIRECTIVES.contains(next) {
            return Err(format!(
                "time specifier {spec:?} has unknown directive '%{next}'"
            ));
        }
    }
    Ok(())
}

impl Display {
    /// Check the block against the rules of the display design.
    pub fn validate(&self) -> Result<(), String> {
        match (&self.number, &self.time) {
            (Some(_), Some(_)) => return Err("display has both number and time".into()),
            (None, None) => return Err("display has neither number nor time".into()),
            _ => {}
        }
        let mut type_char = "";
        if let Some(n) = &self.number {
            if n.contains('$') {
                return Err(format!("number specifier {n:?} contains '$'"));
            }
            let caps = number_re()
                .captures(n)
                .ok_or_else(|| format!("number specifier {n:?} is not a d3-format specifier"))?;
            type_char = caps.get(1).map_or("", |m| m.as_str());
        }
        if let Some(t) = &self.time {
            if t.contains('$') {
                return Err(format!("time specifier {t:?} contains '$'"));
            }
            validate_time(t)?;
        }
        let flagged = self.money || self.currency.is_some() || self.fraction || self.bytes;
        if flagged && self.number.is_none() {
            return Err("money, currency, fraction and bytes need a number specifier".into());
        }
        if let Some(c) = &self.currency {
            if !self.money {
                return Err("currency needs money".into());
            }
            if c.len() != 3 || !c.bytes().all(|b| b.is_ascii_uppercase()) {
                return Err(format!("currency {c:?} is not three uppercase letters"));
            }
        }
        if self.fraction {
            if !matches!(type_char, "%" | "p") {
                return Err("fraction needs a '%' or 'p' number type".into());
            }
            if self.money || self.bytes {
                return Err("fraction excludes money and bytes".into());
            }
        }
        if self.bytes && self.money {
            return Err("bytes and money exclude each other".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn num(s: &str) -> Display {
        Display {
            number: Some(s.into()),
            ..Default::default()
        }
    }
    fn time(s: &str) -> Display {
        Display {
            time: Some(s.into()),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_spec_examples() {
        let ok = |d: Display| assert_eq!(d.validate(), Ok(()), "{d:?}");
        ok(Display {
            money: true,
            ..num("(,.2f")
        });
        ok(Display {
            fraction: true,
            ..num("~%")
        });
        ok(Display {
            bytes: true,
            ..num(",d")
        });
        ok(Display {
            money: true,
            currency: Some("INR".into()),
            ..num(",.2f")
        });
        for s in ["d", "~e", "~s", ",", ",.8~f"] {
            ok(num(s));
        }
        for s in [
            "%d/%m/%Y",
            "%Y-%m-%dT%H:%M:%S.%L%Z",
            "Q%q %Y",
            "%Y年%-m月%-d日",
            "%a %b %-d %H:%M:%S %Y",
        ] {
            ok(time(s));
        }
    }

    #[test]
    fn refuses() {
        let bad = |d: Display| assert!(d.validate().is_err(), "{d:?}");
        bad(Display {
            time: Some("%Y".into()),
            ..num("d")
        });
        bad(Display::default());
        bad(num("$,.2f"));
        bad(num("$>10,.2f"));
        bad(num(",.2q"));
        bad(time("%Y-%k"));
        bad(time("%-k"));
        bad(time("%Y%"));
        bad(Display {
            fraction: true,
            ..num(",.2f")
        });
        bad(Display {
            currency: Some("INR".into()),
            ..num(",.2f")
        });
        bad(Display {
            money: true,
            currency: Some("inr".into()),
            ..num(",.2f")
        });
        bad(Display {
            money: true,
            ..time("%Y")
        });
        bad(Display {
            bytes: true,
            money: true,
            ..num(",d")
        });
        bad(Display {
            fraction: true,
            money: true,
            ..num("~%")
        });
    }

    #[test]
    fn unknown_key_refused() {
        assert!(serde_yaml::from_str::<Display>("numbr: d").is_err());
    }

    #[test]
    fn round_trips_snake_case() {
        let d = Display {
            bytes: true,
            ..num(",d")
        };
        assert_eq!(
            serde_json::to_value(d).unwrap(),
            json!({"number": ",d", "bytes": true})
        );
    }
}
