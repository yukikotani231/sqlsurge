//! Name resolver - resolves table and column references

use sqlparser::ast::{
    Expr, ObjectName, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
    TableWithJoins, GroupByExpr, FunctionArguments,
};
use std::collections::HashMap;

use crate::error::{Diagnostic, DiagnosticKind};
use crate::schema::{Catalog, QualifiedName, TableDef};

/// Resolved table reference in a query
#[derive(Debug, Clone)]
struct TableRef {
    /// The actual table definition
    table: QualifiedName,
    /// Alias used in the query (if any)
    #[allow(dead_code)]
    alias: Option<String>,
}

/// Name resolver for SQL queries
pub struct NameResolver<'a> {
    catalog: &'a Catalog,
    /// Current scope's table references (alias/name -> TableRef)
    tables: HashMap<String, TableRef>,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl<'a> NameResolver<'a> {
    pub fn new(catalog: &'a Catalog) -> Self {
        Self {
            catalog,
            tables: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Resolve names in a statement
    pub fn resolve_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Query(query) => self.resolve_query(query),
            Statement::Insert(insert) => {
                // Resolve table name
                let table_name = object_name_to_qualified(&insert.table_name);
                if !self.catalog.table_exists(&table_name) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            DiagnosticKind::TableNotFound,
                            format!("Table '{}' not found", table_name),
                        )
                        .with_help("Check that the table exists in your schema definition"),
                    );
                }
                // TODO: Resolve column names and values
            }
            Statement::Update { table, .. } => {
                // Resolve table name
                let table_name = table_with_joins_to_name(&table.relation);
                if let Some(name) = table_name {
                    if !self.catalog.table_exists(&name) {
                        self.diagnostics.push(
                            Diagnostic::error(
                                DiagnosticKind::TableNotFound,
                                format!("Table '{}' not found", name),
                            )
                            .with_help("Check that the table exists in your schema definition"),
                        );
                    }
                }
                // TODO: Resolve column names in SET clause
            }
            Statement::Delete(delete) => {
                // Resolve table name from the from clause
                match &delete.from {
                    sqlparser::ast::FromTable::WithFromKeyword(tables) => {
                        if let Some(from_table) = tables.first() {
                            let table_name = table_with_joins_to_name(&from_table.relation);
                            if let Some(name) = table_name {
                                if !self.catalog.table_exists(&name) {
                                    self.diagnostics.push(
                                        Diagnostic::error(
                                            DiagnosticKind::TableNotFound,
                                            format!("Table '{}' not found", name),
                                        )
                                        .with_help("Check that the table exists in your schema definition"),
                                    );
                                }
                            }
                        }
                    }
                    sqlparser::ast::FromTable::WithoutKeyword(tables) => {
                        if let Some(from_table) = tables.first() {
                            let table_name = table_with_joins_to_name(&from_table.relation);
                            if let Some(name) = table_name {
                                if !self.catalog.table_exists(&name) {
                                    self.diagnostics.push(
                                        Diagnostic::error(
                                            DiagnosticKind::TableNotFound,
                                            format!("Table '{}' not found", name),
                                        )
                                        .with_help("Check that the table exists in your schema definition"),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Resolve names in a query
    fn resolve_query(&mut self, query: &Query) {
        // Handle CTEs (WITH clause)
        // TODO: Handle CTEs properly

        // Resolve the main query body
        self.resolve_set_expr(&query.body);
    }

    /// Resolve names in a set expression (SELECT, UNION, etc.)
    fn resolve_set_expr(&mut self, set_expr: &SetExpr) {
        match set_expr {
            SetExpr::Select(select) => self.resolve_select(select),
            SetExpr::Query(query) => self.resolve_query(query),
            SetExpr::SetOperation { left, right, .. } => {
                self.resolve_set_expr(left);
                self.resolve_set_expr(right);
            }
            _ => {}
        }
    }

    /// Resolve names in a SELECT statement
    fn resolve_select(&mut self, select: &Select) {
        // First, resolve FROM clause to build table scope
        for table_with_joins in &select.from {
            self.resolve_table_with_joins(table_with_joins);
        }

        // Then resolve SELECT items
        for item in &select.projection {
            self.resolve_select_item(item);
        }

        // Resolve WHERE clause
        if let Some(selection) = &select.selection {
            self.resolve_expr(selection);
        }

        // Resolve GROUP BY
        match &select.group_by {
            GroupByExpr::All(_) => {}
            GroupByExpr::Expressions(exprs, _) => {
                for expr in exprs {
                    self.resolve_expr(expr);
                }
            }
        }

        // Resolve HAVING
        if let Some(having) = &select.having {
            self.resolve_expr(having);
        }
    }

    /// Resolve a table reference in FROM clause
    fn resolve_table_with_joins(&mut self, table: &TableWithJoins) {
        self.resolve_table_factor(&table.relation);

        for join in &table.joins {
            self.resolve_table_factor(&join.relation);
            // TODO: Resolve join condition
        }
    }

    /// Resolve a table factor (table name, subquery, etc.)
    fn resolve_table_factor(&mut self, factor: &TableFactor) {
        match factor {
            TableFactor::Table { name, alias, .. } => {
                let table_name = object_name_to_qualified(name);

                // Check if table exists
                if !self.catalog.table_exists(&table_name) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            DiagnosticKind::TableNotFound,
                            format!("Table '{}' not found", table_name),
                        )
                        .with_help("Check that the table exists in your schema definition"),
                    );
                    return;
                }

                // Register table in scope
                let alias_name = alias.as_ref().map(|a| a.name.value.clone());
                let lookup_name = alias_name.clone().unwrap_or_else(|| table_name.name.clone());

                self.tables.insert(
                    lookup_name,
                    TableRef {
                        table: table_name,
                        alias: alias_name,
                    },
                );
            }
            TableFactor::Derived { subquery, alias, .. } => {
                // Subquery - resolve it but don't add to scope (for now)
                self.resolve_query(subquery);
                // TODO: Handle subquery columns in scope
                if let Some(a) = alias {
                    // Register derived table with alias
                    // For now, we skip column resolution for derived tables
                    let _ = a.name.value.clone();
                }
            }
            _ => {}
        }
    }

    /// Resolve a SELECT item
    fn resolve_select_item(&mut self, item: &SelectItem) {
        match item {
            SelectItem::UnnamedExpr(expr) => self.resolve_expr(expr),
            SelectItem::ExprWithAlias { expr, .. } => self.resolve_expr(expr),
            SelectItem::QualifiedWildcard(name, _) => {
                // table.*
                let table_name = name.0.first().map(|i| i.value.as_str()).unwrap_or("");
                if !self.tables.contains_key(table_name) {
                    self.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::TableNotFound,
                        format!("Table or alias '{}' not found in FROM clause", table_name),
                    ));
                }
            }
            SelectItem::Wildcard(_) => {
                // * - valid if we have at least one table
                if self.tables.is_empty() {
                    self.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::TableNotFound,
                        "SELECT * requires at least one table in FROM clause",
                    ));
                }
            }
        }
    }

    /// Resolve an expression
    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Identifier(ident) => {
                // Simple column name - must exist in one of the tables
                self.resolve_column(None, &ident.value);
            }
            Expr::CompoundIdentifier(idents) => {
                // table.column or schema.table.column
                match idents.as_slice() {
                    [table, column] => {
                        self.resolve_column(Some(&table.value), &column.value);
                    }
                    [_schema, table, column] => {
                        self.resolve_column(Some(&table.value), &column.value);
                    }
                    _ => {}
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::UnaryOp { expr, .. } => {
                self.resolve_expr(expr);
            }
            Expr::Nested(inner) => {
                self.resolve_expr(inner);
            }
            Expr::Function(func) => {
                match &func.args {
                    FunctionArguments::None => {}
                    FunctionArguments::Subquery(_) => {}
                    FunctionArguments::List(args) => {
                        for arg in &args.args {
                            if let sqlparser::ast::FunctionArg::Unnamed(
                                sqlparser::ast::FunctionArgExpr::Expr(e),
                            ) = arg
                            {
                                self.resolve_expr(e);
                            }
                        }
                    }
                }
            }
            Expr::InList { expr, list, .. } => {
                self.resolve_expr(expr);
                for e in list {
                    self.resolve_expr(e);
                }
            }
            Expr::InSubquery { expr, subquery, .. } => {
                self.resolve_expr(expr);
                self.resolve_query(subquery);
            }
            Expr::Between {
                expr, low, high, ..
            } => {
                self.resolve_expr(expr);
                self.resolve_expr(low);
                self.resolve_expr(high);
            }
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                if let Some(op) = operand {
                    self.resolve_expr(op);
                }
                for cond in conditions {
                    self.resolve_expr(cond);
                }
                for result in results {
                    self.resolve_expr(result);
                }
                if let Some(else_r) = else_result {
                    self.resolve_expr(else_r);
                }
            }
            Expr::Subquery(query) => {
                self.resolve_query(query);
            }
            Expr::IsNull(e) | Expr::IsNotNull(e) => {
                self.resolve_expr(e);
            }
            // Literals and other expressions don't need resolution
            _ => {}
        }
    }

    /// Resolve a column reference
    fn resolve_column(&mut self, table_alias: Option<&str>, column_name: &str) {
        if let Some(alias) = table_alias {
            // Qualified column reference (table.column)
            if let Some(table_ref) = self.tables.get(alias) {
                if let Some(table_def) = self.catalog.get_table(&table_ref.table) {
                    if !table_def.column_exists(column_name) {
                        let similar = find_similar_column(table_def, column_name);
                        let mut diag = Diagnostic::error(
                            DiagnosticKind::ColumnNotFound,
                            format!(
                                "Column '{}' not found in table '{}'",
                                column_name, table_ref.table
                            ),
                        );
                        if let Some(suggestion) = similar {
                            diag = diag.with_help(format!("Did you mean '{}'?", suggestion));
                        }
                        self.diagnostics.push(diag);
                    }
                }
            } else {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::TableNotFound,
                    format!("Table or alias '{}' not found in FROM clause", alias),
                ));
            }
        } else {
            // Unqualified column reference - search all tables in scope
            let mut found_in: Vec<&str> = Vec::new();

            for (name, table_ref) in &self.tables {
                if let Some(table_def) = self.catalog.get_table(&table_ref.table) {
                    if table_def.column_exists(column_name) {
                        found_in.push(name);
                    }
                }
            }

            match found_in.len() {
                0 => {
                    // Column not found in any table
                    let mut suggestions = Vec::new();
                    for table_ref in self.tables.values() {
                        if let Some(table_def) = self.catalog.get_table(&table_ref.table) {
                            if let Some(s) = find_similar_column(table_def, column_name) {
                                suggestions.push(s);
                            }
                        }
                    }

                    let mut diag = Diagnostic::error(
                        DiagnosticKind::ColumnNotFound,
                        format!("Column '{}' not found", column_name),
                    );
                    if !suggestions.is_empty() {
                        diag = diag.with_help(format!("Did you mean '{}'?", suggestions[0]));
                    }
                    self.diagnostics.push(diag);
                }
                1 => {
                    // Found in exactly one table - OK
                }
                _ => {
                    // Ambiguous - found in multiple tables
                    self.diagnostics.push(
                        Diagnostic::error(
                            DiagnosticKind::AmbiguousColumn,
                            format!(
                                "Column '{}' is ambiguous (found in tables: {})",
                                column_name,
                                found_in.join(", ")
                            ),
                        )
                        .with_help(format!(
                            "Qualify the column with a table name: {}.{}",
                            found_in[0], column_name
                        )),
                    );
                }
            }
        }
    }

    /// Consume the resolver and return collected diagnostics
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

/// Convert ObjectName to QualifiedName
fn object_name_to_qualified(name: &ObjectName) -> QualifiedName {
    match name.0.as_slice() {
        [table] => QualifiedName::new(&table.value),
        [schema, table] => QualifiedName::with_schema(&schema.value, &table.value),
        [_catalog, schema, table] => QualifiedName::with_schema(&schema.value, &table.value),
        _ => QualifiedName::new(name.to_string()),
    }
}

/// Get table name from TableFactor
fn table_with_joins_to_name(factor: &TableFactor) -> Option<QualifiedName> {
    match factor {
        TableFactor::Table { name, .. } => Some(object_name_to_qualified(name)),
        _ => None,
    }
}

/// Find a similar column name (for suggestions)
fn find_similar_column(table: &TableDef, name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();
    let mut best_match: Option<(usize, &str)> = None;

    for col_name in table.columns.keys() {
        let col_lower = col_name.to_lowercase();
        let distance = levenshtein_distance(&name_lower, &col_lower);

        // Only suggest if reasonably similar (distance <= 3)
        if distance <= 3 {
            if best_match.is_none() || distance < best_match.unwrap().0 {
                best_match = Some((distance, col_name));
            }
        }
    }

    best_match.map(|(_, name)| name.to_string())
}

/// Simple Levenshtein distance implementation
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}
