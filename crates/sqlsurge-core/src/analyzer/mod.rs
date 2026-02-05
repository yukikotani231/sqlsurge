//! SQL analyzer module

mod resolver;

use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

use crate::error::{Diagnostic, DiagnosticKind, Span};
use crate::schema::Catalog;

pub use resolver::NameResolver;

/// SQL Analyzer - validates SQL against a schema catalog
pub struct Analyzer<'a> {
    catalog: &'a Catalog,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Analyzer<'a> {
    pub fn new(catalog: &'a Catalog) -> Self {
        Self {
            catalog,
            diagnostics: Vec::new(),
        }
    }

    /// Analyze a SQL query and return diagnostics
    pub fn analyze(&mut self, sql: &str) -> Vec<Diagnostic> {
        self.diagnostics.clear();

        // Parse the SQL
        let dialect = PostgreSqlDialect {};
        let statements = match Parser::parse_sql(&dialect, sql) {
            Ok(stmts) => stmts,
            Err(e) => {
                self.diagnostics.push(
                    Diagnostic::error(DiagnosticKind::ParseError, format!("Parse error: {}", e))
                        .with_span(Span::new(0, sql.len().min(50))),
                );
                return std::mem::take(&mut self.diagnostics);
            }
        };

        // Analyze each statement
        for stmt in &statements {
            let mut resolver = NameResolver::new(self.catalog);
            resolver.resolve_statement(stmt);
            self.diagnostics.extend(resolver.into_diagnostics());
        }

        std::mem::take(&mut self.diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SchemaBuilder;

    fn setup_catalog() -> Catalog {
        let schema_sql = r#"
            CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                email TEXT
            );

            CREATE TABLE orders (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL,
                total DECIMAL(10, 2)
            );
        "#;

        let mut builder = SchemaBuilder::new();
        builder.parse(schema_sql).unwrap();
        let (catalog, _) = builder.build();
        catalog
    }

    #[test]
    fn test_valid_select() {
        let catalog = setup_catalog();
        let mut analyzer = Analyzer::new(&catalog);

        let diagnostics = analyzer.analyze("SELECT id, name FROM users");
        assert!(
            diagnostics.is_empty(),
            "Expected no errors: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_table_not_found() {
        let catalog = setup_catalog();
        let mut analyzer = Analyzer::new(&catalog);

        let diagnostics = analyzer.analyze("SELECT * FROM nonexistent");
        // Table not found error should be first
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].kind, DiagnosticKind::TableNotFound);
    }

    #[test]
    fn test_column_not_found() {
        let catalog = setup_catalog();
        let mut analyzer = Analyzer::new(&catalog);

        let diagnostics = analyzer.analyze("SELECT nonexistent_column FROM users");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::ColumnNotFound);
    }
}
