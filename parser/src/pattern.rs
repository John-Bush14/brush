use anyhow::Result;

pub enum ExtendedGlobKind {
    Plus,
    At,
    Exclamation,
    Question,
    Star,
}

pub fn pattern_to_regex_str(pattern: &str, enable_extended_globbing: bool) -> Result<String> {
    let regex_str = pattern_to_regex_translator::pattern(pattern, enable_extended_globbing)?;
    Ok(regex_str)
}

peg::parser! {
    grammar pattern_to_regex_translator(enable_extended_globbing: bool) for str {
        pub(crate) rule pattern() -> String =
            pieces:(pattern_piece()*) {
                pieces.join("")
            }

        rule pattern_piece() -> String =
            escape_sequence() /
            bracket_expression() /
            extglob_enabled() s:extended_glob_pattern() { s } /
            wildcard() /
            [c if needs_escaping(c)] {
                let mut s = '\\'.to_string();
                s.push(c);
                s
            } /
            [c] { c.to_string() }

        rule escape_sequence() -> String =
            "\\" c:[_] { c.to_string() }

        rule bracket_expression() -> String =
            "[" invert:(("!")?) members:bracket_member()+ "]" {
                let mut members = members;
                if invert.is_some() {
                    members.insert(0, String::from("^"));
                }
                members.join("")
            }

        rule bracket_member() -> String =
            char_class_expression() /
            char_range()

        rule char_class_expression() -> String =
            e:$("[:" char_class() ":]") { e.to_owned() }

        rule char_class() =
            "alnum" / "alpha" / "blank" / "cntrl" / "digit" / "graph" / "lower" / "print" / "punct" / "space" / "upper"/ "xdigit"

        rule char_range() -> String =
            range:$([_] "-" [_]) { range.to_owned() }

        rule wildcard() -> String =
            "?" { String::from(".") } /
            "*" { String::from(".*") }

        rule extglob_enabled() -> () =
            &[_] {? if enable_extended_globbing { Ok(()) } else { Err("extglob disabled") } }

        pub(crate) rule extended_glob_pattern() -> String =
            kind:extended_glob_prefix() "(" branches:extended_glob_body() ")" {
                let mut s = String::new();

                s.push('(');

                if matches!(kind, ExtendedGlobKind::Exclamation) {
                    s.push_str("?!");
                }

                s.push_str(&branches.join("|"));
                s.push(')');

                match kind {
                    ExtendedGlobKind::Plus => s.push('+'),
                    ExtendedGlobKind::Question => s.push('?'),
                    ExtendedGlobKind::Star => s.push('*'),
                    ExtendedGlobKind::At | ExtendedGlobKind::Exclamation => (),
                }

                if matches!(kind, ExtendedGlobKind::Exclamation) {
                    s.push_str(".*?");
                }

                s
            }

        rule extended_glob_prefix() -> ExtendedGlobKind =
            "+" { ExtendedGlobKind::Plus } /
            "@" { ExtendedGlobKind::At } /
            "!" { ExtendedGlobKind::Exclamation } /
            "?" { ExtendedGlobKind::Question } /
            "*" { ExtendedGlobKind::Star }

        pub(crate) rule extended_glob_body() -> Vec<String> =
            first_branches:((b:extended_glob_branch() "|" { b })*) last_branch:extended_glob_branch() {
                let mut branches = first_branches;
                branches.push(last_branch);
                branches
            }

        rule extended_glob_branch() -> String =
            pieces:(!['|' | ')'] piece:pattern_piece() { piece })* { pieces.join("") }
    }
}

fn needs_escaping(c: char) -> bool {
    matches!(
        c,
        '[' | ']' | '(' | ')' | '{' | '}' | '*' | '?' | '.' | '+' | '^' | '$' | '|' | '\\'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_glob() -> Result<()> {
        assert_eq!(
            pattern_to_regex_translator::extended_glob_pattern("@(a|b)", true)?,
            "(a|b)"
        );

        assert_eq!(
            pattern_to_regex_translator::extended_glob_body("ab|ac", true)?,
            vec!["ab", "ac"],
        );

        assert_eq!(
            pattern_to_regex_translator::extended_glob_pattern("*(ab|ac)", true)?,
            "(ab|ac)*"
        );

        Ok(())
    }
}
