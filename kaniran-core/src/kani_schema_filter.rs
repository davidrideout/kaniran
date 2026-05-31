//! Rust-only sidecar
//! schema dump helpers
//! The dump is `pg_dump --schema-only --clean --if-exists` output;
fn is_session_preamble(stmt: &str) -> bool {
    let trimmed = stmt.trim_start();
    trimmed.starts_with("SET ") || trimmed.starts_with("SELECT pg_catalog")
}

fn references_any_table(stmt: &str, table_names: &[&str]) -> bool {
    for name in table_names {
        let qualified = format!("public.{name}");
        if stmt.contains(&qualified) {
            return true;
        }
        let qualified_quoted = format!("public.\"{name}\"");
        if stmt.contains(&qualified_quoted) {
            return true;
        }
    }
    false
}

pub fn iter_relevant_statements(
    schema_sql: &str,
    table_names: &[&str],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in split_top_level(schema_sql) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_session_preamble(trimmed) || references_any_table(trimmed, table_names) {
            out.push(trimmed.to_string());
        }
    }
    out
}

/// Splits the database schema dump into individual statements
fn split_top_level(sql: &str) -> Vec<String> {
    let mut statements: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(c) = chars.next() {
        if in_line_comment {
            current.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if in_block_comment {
            current.push(c);
            if c == '*' && chars.peek() == Some(&'/') {
                current.push(chars.next().unwrap());
                in_block_comment = false;
            }
            continue;
        }
        if in_string {
            current.push(c);
            if c == '\'' {
                // Doubled single-quote escape inside a string.
                if chars.peek() == Some(&'\'') {
                    current.push(chars.next().unwrap());
                } else {
                    in_string = false;
                }
            }
            continue;
        }
        match c {
            '\'' => {
                in_string = true;
                current.push(c);
            }
            '-' if chars.peek() == Some(&'-') => {
                in_line_comment = true;
                current.push(c);
                current.push(chars.next().unwrap());
            }
            '/' if chars.peek() == Some(&'*') => {
                in_block_comment = true;
                current.push(c);
                current.push(chars.next().unwrap());
            }
            ';' => {
                let stmt = std::mem::take(&mut current);
                statements.push(stmt);
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        statements.push(current);
    }
    statements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_handles_strings_and_comments() {
        let sql = "SET search_path = '';\n\
                   -- a comment with ; inside\n\
                   CREATE TABLE foo (s text DEFAULT 'a;b');\n\
                   /* block ; comment */\n\
                   CREATE INDEX bar ON foo (s);";
        let stmts = split_top_level(sql);
        // 3 statements — the `;`s inside the `--` line comment, the
        // `'a;b'` string literal, and the `/* … */` block comment are
        // all suppressed; only the three top-level `;` terminators
        // count.
        assert_eq!(stmts.len(), 3, "{stmts:#?}");
        assert!(stmts[0].contains("SET search_path"));
        assert!(stmts[1].contains("CREATE TABLE foo"));
        assert!(stmts[1].contains("a;b"));
        assert!(stmts[2].contains("/* block"));
        assert!(stmts[2].contains("CREATE INDEX bar"));
    }

    #[test]
    fn preamble_always_passes() {
        let sql = "SET client_encoding = 'UTF8';\n\
                   SELECT pg_catalog.set_config('search_path', '', false);\n\
                   CREATE TABLE public.unrelated (i integer);";
        let stmts = iter_relevant_statements(sql, &["other_table"]);
        assert_eq!(stmts.len(), 2, "{stmts:#?}");
        assert!(stmts[0].contains("SET client_encoding"));
        assert!(stmts[1].contains("SELECT pg_catalog"));
    }

    #[test]
    fn selects_only_relevant_tables() {
        let sql = "CREATE TABLE public.foo (i integer);\n\
                   CREATE TABLE public.bar (i integer);\n\
                   ALTER TABLE ONLY public.foo ADD CONSTRAINT k FOREIGN KEY (i) REFERENCES public.bar(i);\n\
                   CREATE INDEX bar_i ON public.bar (i);";
        let only_foo = iter_relevant_statements(sql, &["foo"]);
        assert_eq!(only_foo.len(), 2);
        assert!(only_foo[0].contains("CREATE TABLE public.foo"));
        assert!(only_foo[1].contains("ALTER TABLE ONLY public.foo"));
    }
}
