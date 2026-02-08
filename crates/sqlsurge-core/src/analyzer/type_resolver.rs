//! Type resolver - infers and validates types in SQL expressions

use sqlparser::ast::{BinaryOperator, Expr, Query, Select, Spanned, Statement, Value};
use std::collections::HashMap;

use crate::error::{Diagnostic, DiagnosticKind, Span};
use crate::schema::{Catalog, QualifiedName};
use crate::types::{SqlType, TypeCompatibility};

use super::resolver::NameResolver;

/// Expression type inference result
#[derive(Debug, Clone, PartialEq)]
enum ExpressionType {
    /// Type is known (successfully inferred)
    Known(SqlType),
    /// Type is unknown (e.g., subquery, complex expression)
    Unknown,
    /// Error occurred during type inference
    Error,
}

/// Reference to a table available in the current scope
#[derive(Debug, Clone)]
struct TableRef {
    /// Qualified table name in catalog
    table_name: QualifiedName,
    /// If this is a VIEW, the column names from the view definition
    view_columns: Option<Vec<String>>,
    /// If this is a derived table, the inferred column names
    derived_columns: Option<Vec<String>>,
}

/// Type resolver for SQL expressions
pub struct TypeResolver<'a> {
    catalog: &'a Catalog,
    /// Current scope's table references (alias or name -> TableRef)
    tables: HashMap<String, TableRef>,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl<'a> TypeResolver<'a> {
    /// Create a new type resolver
    pub fn new(catalog: &'a Catalog) -> Self {
        Self {
            catalog,
            tables: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Inherit scope from a NameResolver
    /// This allows TypeResolver to access the same table context as NameResolver
    pub fn inherit_scope(&mut self, resolver: &NameResolver) {
        // Copy table references from NameResolver
        for (key, name_table_ref) in &resolver.tables {
            let type_table_ref = TableRef {
                table_name: name_table_ref.table.clone(),
                view_columns: name_table_ref.view_columns.clone(),
                derived_columns: name_table_ref.derived_columns.clone(),
            };
            self.tables.insert(key.clone(), type_table_ref);
        }
    }

    /// Check types in a statement
    pub fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Query(query) => {
                self.check_query(query);
            }
            Statement::Insert { .. } => {
                // TODO: Check INSERT value types against column types
            }
            Statement::Update { selection, .. } => {
                // TODO: Check SET assignments and WHERE condition types
                if let Some(expr) = selection {
                    self.check_expr_recursive(expr);
                }
            }
            Statement::Delete(delete) => {
                // TODO: Check WHERE condition types
                if let Some(ref selection) = delete.selection {
                    self.check_expr_recursive(selection);
                }
            }
            _ => {}
        }
    }

    /// Check types in a query
    fn check_query(&mut self, query: &Query) {
        // Check the main body
        if let sqlparser::ast::SetExpr::Select(select) = &*query.body {
            self.check_select(select);
        }
        // TODO: Handle UNION, INTERSECT, EXCEPT
    }

    /// Check types in a SELECT statement
    fn check_select(&mut self, select: &Select) {
        // Check JOIN conditions
        for table_with_joins in &select.from {
            for join in &table_with_joins.joins {
                self.check_join_condition(join);
            }
        }

        // Check SELECT projection
        for select_item in &select.projection {
            match select_item {
                sqlparser::ast::SelectItem::UnnamedExpr(expr)
                | sqlparser::ast::SelectItem::ExprWithAlias { expr, .. } => {
                    self.check_expr_recursive(expr);
                }
                sqlparser::ast::SelectItem::QualifiedWildcard(_, _)
                | sqlparser::ast::SelectItem::Wildcard(_) => {
                    // Wildcards don't need type checking
                }
            }
        }

        // Check WHERE clause
        if let Some(ref selection) = select.selection {
            self.check_expr_recursive(selection);
        }

        // TODO: Check HAVING, GROUP BY, etc.
    }

    /// Check types in a JOIN condition
    fn check_join_condition(&mut self, join: &sqlparser::ast::Join) {
        use sqlparser::ast::{JoinConstraint, JoinOperator};

        // Extract the constraint from the join operator
        let constraint = match &join.join_operator {
            JoinOperator::Inner(c) | JoinOperator::LeftOuter(c) | JoinOperator::RightOuter(c) | JoinOperator::FullOuter(c) => c,
            JoinOperator::CrossJoin | JoinOperator::CrossApply | JoinOperator::OuterApply => {
                return; // No condition to check
            }
            _ => return,
        };

        if let JoinConstraint::On(expr) = constraint {
            // Check JOIN ON condition with special handling for top-level comparison
            self.check_join_on_expr(expr);
        }
    }

    /// Check expression in JOIN ON clause
    /// Top-level comparison operators get JoinTypeMismatch error instead of TypeMismatch
    fn check_join_on_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                if self.is_comparison_operator(op) {
                    // This is a comparison in JOIN ON - use JoinTypeMismatch error
                    let left_type = self.infer_expr_type(left);
                    let right_type = self.infer_expr_type(right);

                    if let (ExpressionType::Known(lt), ExpressionType::Known(rt)) =
                        (left_type, right_type)
                    {
                        // Check compatibility in both directions (comparison is symmetric)
                        let compat_lr = lt.is_compatible_with(&rt);
                        let compat_rl = rt.is_compatible_with(&lt);

                        // If either direction allows implicit cast, the comparison is valid
                        if compat_lr == TypeCompatibility::ExplicitCast
                            && compat_rl == TypeCompatibility::ExplicitCast {
                            let span = Span::from_sqlparser(&left.span());
                            self.diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticKind::JoinTypeMismatch,
                                    format!(
                                        "JOIN condition type mismatch: {} vs {}",
                                        lt.display_name(),
                                        rt.display_name()
                                    ),
                                )
                                .with_span(span)
                                .with_help(
                                    "JOIN condition should compare compatible types. Consider using explicit CAST.",
                                ),
                            );
                        }
                    }
                    // Recursively check subexpressions
                    self.check_join_on_expr(left);
                    self.check_join_on_expr(right);
                } else {
                    // Non-comparison operator - check recursively
                    self.check_join_on_expr(left);
                    self.check_join_on_expr(right);
                }
            }
            Expr::Nested(inner) => {
                self.check_join_on_expr(inner);
            }
            _ => {
                // Leaf expressions - no further checking needed
            }
        }
    }

    /// Check if an operator is a comparison operator
    fn is_comparison_operator(&self, op: &BinaryOperator) -> bool {
        matches!(
            op,
            BinaryOperator::Eq
                | BinaryOperator::NotEq
                | BinaryOperator::Lt
                | BinaryOperator::LtEq
                | BinaryOperator::Gt
                | BinaryOperator::GtEq
        )
    }

    /// Recursively check types in an expression
    fn check_expr_recursive(&mut self, expr: &Expr) {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                // Check the binary operation
                self.check_binary_op(left, op, right);
                // Recursively check subexpressions
                self.check_expr_recursive(left);
                self.check_expr_recursive(right);
            }
            Expr::Nested(inner) => {
                self.check_expr_recursive(inner);
            }
            Expr::UnaryOp { expr, .. } => {
                self.check_expr_recursive(expr);
            }
            Expr::InList { expr, list, .. } => {
                self.check_expr_recursive(expr);
                for item in list {
                    self.check_expr_recursive(item);
                }
            }
            Expr::Between {
                expr, low, high, ..
            } => {
                self.check_expr_recursive(expr);
                self.check_expr_recursive(low);
                self.check_expr_recursive(high);
            }
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                if let Some(op) = operand {
                    self.check_expr_recursive(op);
                }
                for cond in conditions {
                    self.check_expr_recursive(cond);
                }
                for res in results {
                    self.check_expr_recursive(res);
                }
                if let Some(else_res) = else_result {
                    self.check_expr_recursive(else_res);
                }
            }
            _ => {
                // Base case: leaf expressions like identifiers, literals
            }
        }
    }

    /// Check type compatibility in a binary operation
    fn check_binary_op(&mut self, left: &Expr, op: &BinaryOperator, right: &Expr) {
        let left_type = self.infer_expr_type(left);
        let right_type = self.infer_expr_type(right);

        // Only check if both types are known
        if let (ExpressionType::Known(lt), ExpressionType::Known(rt)) = (left_type, right_type) {
            match op {
                // Comparison operators
                BinaryOperator::Eq
                | BinaryOperator::NotEq
                | BinaryOperator::Lt
                | BinaryOperator::LtEq
                | BinaryOperator::Gt
                | BinaryOperator::GtEq => {
                    // Check compatibility in both directions (comparison is symmetric)
                    let compat_lr = lt.is_compatible_with(&rt);
                    let compat_rl = rt.is_compatible_with(&lt);

                    // If either direction allows implicit cast, the comparison is valid
                    if compat_lr == TypeCompatibility::ExplicitCast
                        && compat_rl == TypeCompatibility::ExplicitCast {
                        // Types are not implicitly compatible in either direction
                        let span = Span::from_sqlparser(&left.span());
                        self.diagnostics.push(
                            Diagnostic::error(
                                DiagnosticKind::TypeMismatch,
                                format!(
                                    "Type mismatch: cannot compare {} with {}",
                                    lt.display_name(),
                                    rt.display_name()
                                ),
                            )
                            .with_span(span)
                            .with_help("Types are not implicitly compatible. Consider using explicit CAST."),
                        );
                    }
                }
                // Arithmetic operators
                BinaryOperator::Plus
                | BinaryOperator::Minus
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo => {
                    // Check if both types are numeric
                    if !self.is_numeric_type(&lt) {
                        let span = Span::from_sqlparser(&left.span());
                        self.diagnostics.push(
                            Diagnostic::error(
                                DiagnosticKind::TypeMismatch,
                                format!(
                                    "Arithmetic operation requires numeric types, but got {}",
                                    lt.display_name()
                                ),
                            )
                            .with_span(span),
                        );
                    }
                    if !self.is_numeric_type(&rt) {
                        let span = Span::from_sqlparser(&right.span());
                        self.diagnostics.push(
                            Diagnostic::error(
                                DiagnosticKind::TypeMismatch,
                                format!(
                                    "Arithmetic operation requires numeric types, but got {}",
                                    rt.display_name()
                                ),
                            )
                            .with_span(span),
                        );
                    }
                }
                // String concatenation operator
                BinaryOperator::StringConcat => {
                    // PostgreSQL || operator - typically used with strings
                    // For now, we allow any type (many types can be cast to string)
                }
                _ => {
                    // Other operators (AND, OR, bitwise, etc.) - skip for now
                }
            }
        }
    }

    /// Check if a type is numeric
    fn is_numeric_type(&self, sql_type: &SqlType) -> bool {
        matches!(
            sql_type,
            SqlType::TinyInt
                | SqlType::SmallInt
                | SqlType::MediumInt
                | SqlType::Integer
                | SqlType::BigInt
                | SqlType::Real
                | SqlType::DoublePrecision
                | SqlType::Decimal { .. }
        )
    }

    /// Consume the resolver and return collected diagnostics
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Infer the type of an expression
    fn infer_expr_type(&mut self, expr: &Expr) -> ExpressionType {
        match expr {
            Expr::Value(value) => self.infer_literal_type(value),
            Expr::Identifier(ident) => self.infer_column_type_from_ident(&ident.value),
            Expr::CompoundIdentifier(parts) => {
                if parts.len() == 2 {
                    // table.column
                    self.infer_column_type_qualified(&parts[0].value, &parts[1].value)
                } else {
                    // More complex identifier (schema.table.column)
                    ExpressionType::Unknown
                }
            }
            _ => ExpressionType::Unknown,
        }
    }

    /// Infer type from a literal value
    fn infer_literal_type(&self, value: &Value) -> ExpressionType {
        match value {
            Value::Number(_, _) => {
                // Simplified: all numbers are integers for now
                // Future: distinguish between integer and decimal based on presence of '.'
                ExpressionType::Known(SqlType::Integer)
            }
            Value::SingleQuotedString(_) | Value::DoubleQuotedString(_) => {
                ExpressionType::Known(SqlType::Text)
            }
            Value::Boolean(_) => ExpressionType::Known(SqlType::Boolean),
            Value::Null => {
                // NULL can be any type (compatible with everything)
                ExpressionType::Unknown
            }
            _ => ExpressionType::Unknown,
        }
    }

    /// Infer type from an unqualified column identifier
    fn infer_column_type_from_ident(&self, col_name: &str) -> ExpressionType {
        // Search through all tables in scope to find the column
        let mut found_type: Option<SqlType> = None;

        for table_ref in self.tables.values() {
            // Check if this is a derived table or view
            if let Some(ref derived_cols) = table_ref.derived_columns {
                if derived_cols.contains(&col_name.to_string()) {
                    // Column exists in derived table, but we don't know its type
                    return ExpressionType::Unknown;
                }
            } else if let Some(ref view_cols) = table_ref.view_columns {
                if view_cols.contains(&col_name.to_string()) {
                    // Column exists in view, but we don't know its type without analyzing the view
                    return ExpressionType::Unknown;
                }
            } else {
                // Regular table - look up in catalog
                if let Some(table_def) = self.catalog.get_table(&table_ref.table_name) {
                    if let Some(col_def) = table_def.get_column(col_name) {
                        if found_type.is_some() {
                            // Column is ambiguous (exists in multiple tables)
                            return ExpressionType::Unknown;
                        }
                        found_type = Some(col_def.data_type.clone());
                    }
                }
            }
        }

        found_type.map_or(ExpressionType::Unknown, ExpressionType::Known)
    }

    /// Infer type from a qualified column identifier (table.column)
    fn infer_column_type_qualified(
        &self,
        table_name: &str,
        col_name: &str,
    ) -> ExpressionType {
        // Look up table in scope
        if let Some(table_ref) = self.tables.get(table_name) {
            // Check if this is a derived table or view
            if table_ref.derived_columns.is_some() || table_ref.view_columns.is_some() {
                // We can't infer types for derived tables or views yet
                return ExpressionType::Unknown;
            }

            // Regular table - look up in catalog
            if let Some(table_def) = self.catalog.get_table(&table_ref.table_name) {
                if let Some(col_def) = table_def.get_column(col_name) {
                    return ExpressionType::Known(col_def.data_type.clone());
                }
            }
        }

        ExpressionType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SchemaBuilder;

    #[test]
    fn test_infer_literal_number() {
        let catalog = Catalog::default();
        let resolver = TypeResolver::new(&catalog);
        let value = Value::Number("123".to_string(), false);
        let result = resolver.infer_literal_type(&value);
        assert_eq!(result, ExpressionType::Known(SqlType::Integer));
    }

    #[test]
    fn test_infer_literal_string() {
        let catalog = Catalog::default();
        let resolver = TypeResolver::new(&catalog);
        let value = Value::SingleQuotedString("hello".to_string());
        let result = resolver.infer_literal_type(&value);
        assert_eq!(result, ExpressionType::Known(SqlType::Text));
    }

    #[test]
    fn test_infer_literal_boolean() {
        let catalog = Catalog::default();
        let resolver = TypeResolver::new(&catalog);
        let value = Value::Boolean(true);
        let result = resolver.infer_literal_type(&value);
        assert_eq!(result, ExpressionType::Known(SqlType::Boolean));
    }

    #[test]
    fn test_infer_literal_null() {
        let catalog = Catalog::default();
        let resolver = TypeResolver::new(&catalog);
        let value = Value::Null;
        let result = resolver.infer_literal_type(&value);
        assert_eq!(result, ExpressionType::Unknown);
    }

    #[test]
    fn test_type_mismatch_comparison() {
        let schema_sql = "CREATE TABLE users (id INTEGER, name TEXT);";
        let mut builder = SchemaBuilder::new();
        builder.parse(schema_sql).unwrap();
        let (catalog, _) = builder.build();

        // Parse the query
        let dialect = crate::dialect::SqlDialect::PostgreSQL.parser_dialect();
        let statements =
            sqlparser::parser::Parser::parse_sql(dialect.as_ref(), "SELECT * FROM users WHERE id = 'text'")
                .unwrap();

        let mut name_resolver = super::super::resolver::NameResolver::new(&catalog);
        name_resolver.resolve_statement(&statements[0]);

        let mut type_resolver = TypeResolver::new(&catalog);
        type_resolver.inherit_scope(&name_resolver);
        type_resolver.check_statement(&statements[0]);

        let diagnostics = type_resolver.into_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::TypeMismatch);
        assert!(diagnostics[0].message.contains("integer"));
        assert!(diagnostics[0].message.contains("text"));
    }

    #[test]
    fn test_arithmetic_on_text() {
        let schema_sql = "CREATE TABLE users (id INTEGER, name TEXT);";
        let mut builder = SchemaBuilder::new();
        builder.parse(schema_sql).unwrap();
        let (catalog, _) = builder.build();

        let dialect = crate::dialect::SqlDialect::PostgreSQL.parser_dialect();
        let statements =
            sqlparser::parser::Parser::parse_sql(dialect.as_ref(), "SELECT name + 10 FROM users")
                .unwrap();

        let mut name_resolver = super::super::resolver::NameResolver::new(&catalog);
        name_resolver.resolve_statement(&statements[0]);

        let mut type_resolver = TypeResolver::new(&catalog);
        type_resolver.inherit_scope(&name_resolver);
        type_resolver.check_statement(&statements[0]);

        let diagnostics = type_resolver.into_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::TypeMismatch);
        assert!(diagnostics[0].message.contains("text"));
    }

    #[test]
    fn test_join_type_mismatch() {
        let schema_sql = r#"
            CREATE TABLE users (id INTEGER, name TEXT);
            CREATE TABLE orders (order_id INTEGER, user_name TEXT);
        "#;
        let mut builder = SchemaBuilder::new();
        builder.parse(schema_sql).unwrap();
        let (catalog, _) = builder.build();

        let dialect = crate::dialect::SqlDialect::PostgreSQL.parser_dialect();
        let statements = sqlparser::parser::Parser::parse_sql(
            dialect.as_ref(),
            "SELECT * FROM users JOIN orders ON users.id = orders.user_name",
        )
        .unwrap();

        let mut name_resolver = super::super::resolver::NameResolver::new(&catalog);
        name_resolver.resolve_statement(&statements[0]);

        let mut type_resolver = TypeResolver::new(&catalog);
        type_resolver.inherit_scope(&name_resolver);
        type_resolver.check_statement(&statements[0]);

        let diagnostics = type_resolver.into_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::JoinTypeMismatch);
        assert!(diagnostics[0].message.contains("integer"));
        assert!(diagnostics[0].message.contains("text"));
    }
}
