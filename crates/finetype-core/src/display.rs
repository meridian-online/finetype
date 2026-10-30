//! The `display` block on a taxonomy definition: how a type's values are shown.
//!
//! A block carries either a d3-format specifier (`number`) or a d3-time-format
//! specifier (`time`), plus flags for facts a specifier cannot express. It
//! holds only what the label implies; scale and precision stay with the caller.

use crate::taxonomy::Definition;
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

/// What a definition's values are, as far as display goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    Number,
    Time,
}

/// Keys whose kind the storage type does not give: a snowflake id is a BIGINT
/// that encodes a moment.
pub const KIND_OVERRIDES: &[(&str, DisplayKind)] =
    &[("technology.identifier.snowflake_id", DisplayKind::Time)];

fn numeric_transform_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)AS\s+(DECIMAL|DOUBLE|INTEGER|BIGINT|SMALLINT)(\(\s*\d+\s*,\s*\d+\s*\))?\s*\)\s*(/\s*\d+)?$",
        )
        .expect("numeric transform regex")
    })
}

fn fraction_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(/\s*(100|10000)|\|\|\s*'e-(2|4)'\s+AS\s+DOUBLE\s*\))\s*$")
            .expect("fraction transform regex")
    })
}

/// Which kind of display block a definition takes, if any.
pub fn display_kind_for(key: &str, def: &Definition) -> Option<DisplayKind> {
    if let Some((_, kind)) = KIND_OVERRIDES.iter().find(|(k, _)| *k == key) {
        return Some(*kind);
    }
    match def.broad_type.as_deref()?.to_ascii_uppercase().as_str() {
        "SMALLINT" | "INTEGER" | "BIGINT" | "HUGEINT" | "DECIMAL" | "DOUBLE" | "FLOAT" | "REAL" => {
            Some(DisplayKind::Number)
        }
        "DATE" | "TIME" | "TIMESTAMP" | "TIMESTAMPTZ" => Some(DisplayKind::Time),
        "VARCHAR" => {
            let t = def.transform.as_deref()?.trim();
            numeric_transform_re()
                .is_match(t)
                .then_some(DisplayKind::Number)
        }
        _ => None,
    }
}

/// Check one definition's display block against its kind. `require` makes a
/// missing block an error.
pub fn check_display(key: &str, def: &Definition, require: bool) -> Result<(), String> {
    let kind = display_kind_for(key, def);
    let Some(d) = &def.display else {
        return match (kind, require) {
            (Some(_), true) => Err("missing `display` block".into()),
            _ => Ok(()),
        };
    };
    let Some(kind) = kind else {
        return Err("has a `display` block but is neither a number nor a time".into());
    };
    d.validate()?;
    match kind {
        DisplayKind::Number if d.number.is_none() => {
            return Err("is a number but its display block has no `number`".into())
        }
        DisplayKind::Time if d.time.is_none() => {
            return Err("is a time but its display block has no `time`".into())
        }
        _ => {}
    }
    if d.fraction {
        let t = def.transform.as_deref().unwrap_or("").trim();
        if !fraction_re().is_match(t) {
            return Err(
                "`fraction` needs a transform that divides by 100 or 10000 (or appends e-2 or e-4)"
                    .into(),
            );
        }
    }
    Ok(())
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

    fn def(yaml: &str) -> Definition {
        serde_yaml::from_str(yaml).expect("definition yaml")
    }
    fn kind(broad: &str, transform: Option<&str>) -> Option<DisplayKind> {
        let mut d = def(&format!("broad_type: {broad}"));
        d.transform = transform.map(String::from);
        display_kind_for("a.b.c", &d)
    }

    #[test]
    fn kind_from_broad_type() {
        for b in ["SMALLINT", "BIGINT", "DECIMAL", "DOUBLE"] {
            assert_eq!(kind(b, None), Some(DisplayKind::Number), "{b}");
        }
        for b in ["DATE", "TIME", "TIMESTAMP", "TIMESTAMPTZ"] {
            assert_eq!(kind(b, None), Some(DisplayKind::Time), "{b}");
        }
        for b in ["INTERVAL", "BOOLEAN", "UUID", "JSON", "POINT"] {
            assert_eq!(kind(b, None), None, "{b}");
        }
    }

    #[test]
    fn kind_from_varchar_transform() {
        assert_eq!(
            kind(
                "VARCHAR",
                Some("CAST(REGEXP_REPLACE({col}, '%$', '') AS DECIMAL(10,4)) / 100")
            ),
            Some(DisplayKind::Number)
        );
        assert_eq!(
            kind("VARCHAR", Some("CAST({col} AS INTEGER)")),
            Some(DisplayKind::Number)
        );
        assert_eq!(kind("VARCHAR", Some("CAST({col} AS VARCHAR)")), None);
        assert_eq!(kind("VARCHAR", None), None);
    }

    #[test]
    fn snowflake_override() {
        let d = def("broad_type: BIGINT");
        assert_eq!(
            display_kind_for("technology.identifier.snowflake_id", &d),
            Some(DisplayKind::Time)
        );
    }

    fn with(broad: &str, transform: Option<&str>, disp: Option<Display>) -> Definition {
        let mut d = def(&format!("broad_type: {broad}"));
        d.transform = transform.map(String::from);
        d.display = disp;
        d
    }

    #[test]
    fn check_display_cases() {
        let k = "a.b.c";
        // wrong kind
        assert!(check_display(k, &with("DATE", None, Some(num("d"))), false).is_err());
        assert!(check_display(k, &with("BIGINT", None, Some(time("%Y"))), false).is_err());
        // block on a kind-less definition
        assert!(check_display(k, &with("UUID", None, Some(num("d"))), false).is_err());
        // missing block
        assert!(check_display(k, &with("BIGINT", None, None), true).is_err());
        assert!(check_display(k, &with("BIGINT", None, None), false).is_ok());
        assert!(check_display(k, &with("UUID", None, None), true).is_ok());
        // good blocks
        assert!(check_display(k, &with("BIGINT", None, Some(num(",d"))), true).is_ok());
        assert!(check_display(k, &with("DATE", None, Some(time("%Y"))), true).is_ok());
        // invalid block surfaces the validator's error
        assert!(check_display(k, &with("BIGINT", None, Some(num("$d"))), false).is_err());
    }

    #[test]
    fn fraction_needs_a_scaling_transform() {
        let frac = Some(Display {
            fraction: true,
            ..num("~%")
        });
        let run = |t: Option<&str>| check_display("a.b.c", &with("DOUBLE", t, frac.clone()), false);
        assert!(run(Some("CAST({col} AS DOUBLE) / 100")).is_ok());
        assert!(run(Some("CAST({col} AS DOUBLE) / 10000")).is_ok());
        assert!(run(Some("CAST({col} AS DOUBLE) / 1000")).is_err());
        assert!(run(Some("CAST({col} AS DOUBLE)")).is_err());
        assert!(run(None).is_err());
        assert!(run(Some(
            "CAST(REGEXP_EXTRACT({col}, '^%?(-?[0-9]+)', 1) || 'e-2' AS DOUBLE)"
        ))
        .is_ok());
        assert!(run(Some("CAST({col} || 'e-4'  as double )")).is_ok());
        assert!(run(Some("CAST({col} || 'e-3' AS DOUBLE)")).is_err());
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
