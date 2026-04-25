// The helper functions in this module are intentionally unused until Task 4
// wires format_sql into EditorView.
#![allow(dead_code)]

use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

pub fn format_sql(sql: &str) -> Result<String, String> {
    if sql.trim().is_empty() {
        return Ok(String::new());
    }
    let dialect = GenericDialect {};
    let statements =
        Parser::parse_sql(&dialect, sql).map_err(|e| e.to_string())?;
    let formatted: Vec<String> =
        statements.iter().map(format_statement).collect();
    Ok(formatted.join("\n\n"))
}

fn format_statement(stmt: &Statement) -> String {
    match stmt {
        Statement::Query(q) => format_query(q),
        _ => format!("{stmt}"),
    }
}

fn format_query(query: &Query) -> String {
    let mut parts: Vec<String> = vec![];

    if let Some(with) = &query.with {
        let cte_parts: Vec<String> = with
            .cte_tables
            .iter()
            .map(|cte| {
                let body = format_query(&cte.query);
                format!("{} AS (\n{}\n)", cte.alias.name, add_indent(&body, 4))
            })
            .collect();
        let recursive = if with.recursive { "RECURSIVE " } else { "" };
        parts.push(format!("WITH {}{}", recursive, cte_parts.join(",\n")));
    }

    match query.body.as_ref() {
        SetExpr::Select(select) => parts.push(format_select(select)),
        other => parts.push(format!("{other}")),
    }

    if let Some(order_by) = &query.order_by {
        if !order_by.exprs.is_empty() {
            let items: Vec<String> = order_by
                .exprs
                .iter()
                .map(|o| {
                    let dir = match o.asc {
                        Some(true) => " ASC",
                        Some(false) => " DESC",
                        None => "",
                    };
                    format!("    {}{}", o.expr, dir)
                })
                .collect();
            parts.push(format!("ORDER BY\n{}", items.join(",\n")));
        }
    }

    if let Some(limit) = &query.limit {
        parts.push(format!("LIMIT {limit}"));
    }

    if let Some(offset) = &query.offset {
        parts.push(format!("OFFSET {}", offset.value));
    }

    parts.join("\n")
}

fn format_select(select: &Select) -> String {
    let mut parts: Vec<String> = vec![];

    let keyword = match &select.distinct {
        Some(_) => "SELECT DISTINCT",
        None => "SELECT",
    };

    let cols: Vec<String> = select
        .projection
        .iter()
        .map(|item| format!("    {}", format_select_item(item)))
        .collect();
    parts.push(format!("{}\n{}", keyword, cols.join(",\n")));

    for twj in &select.from {
        parts.push(format!("FROM {}", format_table_with_joins(twj)));
    }

    if let Some(selection) = &select.selection {
        parts.push(format_where("WHERE", selection));
    }

    match &select.group_by {
        GroupByExpr::Expressions(exprs, _) if !exprs.is_empty() => {
            let items: Vec<String> =
                exprs.iter().map(|e| format!("    {e}")).collect();
            parts.push(format!("GROUP BY\n{}", items.join(",\n")));
        }
        _ => {}
    }

    if let Some(having) = &select.having {
        parts.push(format_where("HAVING", having));
    }

    parts.join("\n")
}

fn format_where(keyword: &str, expr: &Expr) -> String {
    let conds = flatten_condition(expr);
    let lines: Vec<String> = conds
        .iter()
        .enumerate()
        .map(|(i, (connector, text))| {
            if i == 0 {
                format!("    {text}")
            } else {
                format!("    {} {text}", connector.as_deref().unwrap_or("AND"))
            }
        })
        .collect();
    format!("{keyword}\n{}", lines.join("\n"))
}

fn format_select_item(item: &SelectItem) -> String {
    match item {
        SelectItem::UnnamedExpr(e) => format!("{e}"),
        SelectItem::ExprWithAlias { expr, alias } => format!("{expr} AS {alias}"),
        SelectItem::QualifiedWildcard(name, _) => format!("{name}.*"),
        SelectItem::Wildcard(_) => "*".to_string(),
    }
}

fn format_table_with_joins(twj: &TableWithJoins) -> String {
    let mut parts = vec![format_table_factor(&twj.relation)];
    for join in &twj.joins {
        parts.push(format_join(join));
    }
    parts.join("\n")
}

fn format_table_factor(tf: &TableFactor) -> String {
    match tf {
        TableFactor::Table { name, alias, .. } => match alias {
            Some(a) => format!("{name} AS {}", a.name),
            None => format!("{name}"),
        },
        TableFactor::Derived {
            subquery, alias, ..
        } => {
            let inner = format_query(subquery);
            let indented = add_indent(&inner, 4);
            match alias {
                Some(a) => format!("(\n{indented}\n) AS {}", a.name),
                None => format!("(\n{indented}\n)"),
            }
        }
        other => format!("{other}"),
    }
}

fn format_join(join: &Join) -> String {
    let (keyword, constraint) = match &join.join_operator {
        JoinOperator::Inner(c) => ("JOIN", Some(c)),
        JoinOperator::LeftOuter(c) => ("LEFT JOIN", Some(c)),
        JoinOperator::RightOuter(c) => ("RIGHT JOIN", Some(c)),
        JoinOperator::FullOuter(c) => ("FULL JOIN", Some(c)),
        JoinOperator::CrossJoin => ("CROSS JOIN", None),
        _ => ("JOIN", None),
    };

    let table = format_table_factor(&join.relation);

    match constraint {
        Some(JoinConstraint::On(expr)) => {
            format!("{keyword} {table}\n    ON {expr}")
        }
        Some(JoinConstraint::Using(cols)) => {
            let names: Vec<String> = cols.iter().map(|c| format!("{c}")).collect();
            format!("{keyword} {table} USING ({})", names.join(", "))
        }
        _ => format!("{keyword} {table}"),
    }
}

/// Flatten a chain of top-level AND/OR into a list of (connector, expression_string) pairs.
/// The first item has `connector = None`; subsequent items carry `Some("AND")` or `Some("OR")`.
fn flatten_condition(expr: &Expr) -> Vec<(Option<String>, String)> {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And,
            right,
        } => {
            let mut parts = flatten_condition(left.as_ref());
            let mut right_parts = flatten_condition(right.as_ref());
            if let Some(first) = right_parts.first_mut() {
                if first.0.is_none() {
                    first.0 = Some("AND".to_string());
                }
            }
            parts.extend(right_parts);
            parts
        }
        Expr::BinaryOp {
            left,
            op: BinaryOperator::Or,
            right,
        } => {
            let mut parts = flatten_condition(left.as_ref());
            let mut right_parts = flatten_condition(right.as_ref());
            if let Some(first) = right_parts.first_mut() {
                if first.0.is_none() {
                    first.0 = Some("OR".to_string());
                }
            }
            parts.extend(right_parts);
            parts
        }
        other => vec![(None, format!("{other}"))],
    }
}

fn add_indent(s: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    s.lines()
        .map(|l| {
            if l.is_empty() {
                l.to_string()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_empty() {
        assert_eq!(format_sql("").unwrap(), "");
    }

    #[test]
    fn test_whitespace_only_returns_empty() {
        assert_eq!(format_sql("   \n  ").unwrap(), "");
    }

    #[test]
    fn test_invalid_sql_returns_err() {
        assert!(format_sql("SELEKT garbage *** from").is_err());
    }

    #[test]
    fn test_simple_select_all() {
        let out = format_sql("select * from users").unwrap();
        assert_eq!(out, "SELECT\n    *\nFROM users");
    }

    #[test]
    fn test_select_columns() {
        let out = format_sql("select id, name from users").unwrap();
        assert_eq!(out, "SELECT\n    id,\n    name\nFROM users");
    }

    #[test]
    fn test_where_single_condition() {
        let out = format_sql("select id from users where active = 1").unwrap();
        assert_eq!(out, "SELECT\n    id\nFROM users\nWHERE\n    active = 1");
    }

    #[test]
    fn test_where_and_conditions() {
        let out = format_sql("select id from users where active = 1 and age > 18").unwrap();
        assert_eq!(
            out,
            "SELECT\n    id\nFROM users\nWHERE\n    active = 1\n    AND age > 18"
        );
    }

    #[test]
    fn test_where_or_conditions() {
        let out = format_sql("select id from users where active = 1 or age > 18").unwrap();
        assert_eq!(
            out,
            "SELECT\n    id\nFROM users\nWHERE\n    active = 1\n    OR age > 18"
        );
    }

    #[test]
    fn test_join_on() {
        let out = format_sql(
            "select u.id from users as u join orders as o on o.user_id = u.id",
        )
        .unwrap();
        assert!(
            out.contains("JOIN orders AS o\n    ON o.user_id = u.id"),
            "actual output:\n{out}"
        );
    }

    #[test]
    fn test_left_join() {
        let out = format_sql(
            "select u.id from users as u left join orders as o on o.user_id = u.id",
        )
        .unwrap();
        assert!(out.contains("LEFT JOIN"), "actual output:\n{out}");
    }

    #[test]
    fn test_order_by() {
        let out = format_sql("select id from users order by id desc").unwrap();
        assert!(out.contains("ORDER BY\n    id DESC"), "actual output:\n{out}");
    }

    #[test]
    fn test_limit() {
        let out = format_sql("select id from users limit 10").unwrap();
        assert!(out.contains("LIMIT 10"), "actual output:\n{out}");
    }

    #[test]
    fn test_group_by_having() {
        let out = format_sql(
            "select user_id, count(*) from orders group by user_id having count(*) > 5",
        )
        .unwrap();
        assert!(out.contains("GROUP BY\n    user_id"), "actual output:\n{out}");
        assert!(out.contains("HAVING"), "actual output:\n{out}");
    }

    #[test]
    fn test_multiple_statements() {
        let out = format_sql("select 1; select 2").unwrap();
        assert!(out.contains("\n\n"), "actual output:\n{out}");
    }

    #[test]
    fn test_alias() {
        let out = format_sql("select id as user_id from users").unwrap();
        assert!(out.contains("id AS user_id"), "actual output:\n{out}");
    }
}
