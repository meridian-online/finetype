//! Structural substance checks — value-shape predicates that recognise a
//! serialization *structure* rather than a set membership or a check digit.
//!
//! These are the guard-owned equivalent of [`crate::membership`] /
//! [`crate::checksum`] for types whose substance is a nesting grammar. The
//! first (and currently only) member is [`is_s_expression`], the balanced
//! nested-parenthesis check behind `container.object.s_expression`.

/// True if `value` is an S-expression: a balanced, recursively-nested
/// parenthetical structure `(head child child ...)`.
///
/// This is the substance check behind the `s_expression_recovery` Sharpen
/// guard. It is deliberately **truncation-tolerant**: a very long parse tree
/// may reach the guard clipped mid-tree, so a value that opens and nests
/// correctly but never closes (final depth > 0) is still accepted — what is
/// rejected is a value that *closes below zero* (an unbalanced `)` with no
/// matching `(`), which no genuine S-expression prefix can do.
///
/// The signature is self-precise on real corpus data (parse trees, code ASTs,
/// Lisp) with zero measured over-recovery, so no header corroboration is
/// needed — unlike the value-ambiguous checksum/membership types.
///
/// Requirements:
/// - after trimming, at least 5 chars and starts with `(`
/// - parentheses never close below zero (balanced, or open if truncated)
/// - maximum nesting depth >= 2 (a flat `(a b c)` list is not enough)
/// - at least 3 opening parens (multiple nodes, not a single `(x (y))`)
pub fn is_s_expression(value: &str) -> bool {
    let t = value.trim();
    if t.len() < 5 || !t.starts_with('(') {
        return false;
    }
    let mut depth: i32 = 0;
    let mut max_depth: i32 = 0;
    let mut opens: u32 = 0;
    for c in t.chars() {
        match c {
            '(' => {
                depth += 1;
                opens += 1;
                if depth > max_depth {
                    max_depth = depth;
                }
            }
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    max_depth >= 2 && opens >= 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_parse_trees_asts_and_lisp() {
        assert!(is_s_expression("(ROOT (S (NP (NN cat)) (VP (VBZ sits))))"));
        assert!(is_s_expression("(program (call (id print) (string hi)))"));
        assert!(is_s_expression("(+ (* 2 3) (- 4 1))"));
        assert!(is_s_expression("(a (b c) (d (e f)))"));
        // Penn comma-token — the shape that fools the comma_separated detector.
        assert!(is_s_expression("(S (INTJ (UH Uh)) (, ,) (NP-SBJ (PRP I)))"));
    }

    #[test]
    fn tolerates_truncation_mid_tree() {
        // Clipped before the closing parens — still a valid open prefix.
        assert!(is_s_expression(
            "(ROOT (SINV (ADVP (RB so)) (, ,) (SBARQ (INTJ (UH uh"
        ));
    }

    #[test]
    fn rejects_non_s_expressions() {
        assert!(!is_s_expression("apple,banana,cherry")); // comma list
        assert!(!is_s_expression("(a b c)")); // flat, depth 1
        assert!(!is_s_expression("hello (world)")); // does not start with (
        assert!(!is_s_expression("(a)) more")); // closes below zero
        assert!(!is_s_expression("(x)")); // too short / not nested
        assert!(!is_s_expression("")); // empty
        assert!(!is_s_expression("{\"a\": 1}")); // JSON, not parens
        assert!(!is_s_expression("(a) (b) (c)")); // three flat groups, depth 1
    }

    #[test]
    fn requires_nesting_not_just_many_parens() {
        // depth 1 throughout despite many opens → not an s-expression
        assert!(!is_s_expression("(a) (b) (c) (d)"));
    }
}
