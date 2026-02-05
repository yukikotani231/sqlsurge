//! Schema builder - converts SQL AST to Catalog

use sqlparser::ast::{ColumnOption, ColumnOptionDef, ObjectName, Statement, TableConstraint};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

use crate::error::{Diagnostic, DiagnosticKind, Span};
use crate::schema::{
    Catalog, ColumnDef, DefaultValue, ForeignKeyDef, PrimaryKeyDef, QualifiedName, TableDef,
    UniqueConstraintDef,
};
use crate::types::SqlType;

/// Builder for constructing a Catalog from SQL schema definitions
pub struct SchemaBuilder {
    catalog: Catalog,
    diagnostics: Vec<Diagnostic>,
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            catalog: Catalog::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Parse SQL schema definitions and build the catalog
    pub fn parse(&mut self, sql: &str) -> Result<(), Vec<Diagnostic>> {
        let dialect = PostgreSqlDialect {};
        let statements = match Parser::parse_sql(&dialect, sql) {
            Ok(stmts) => stmts,
            Err(e) => {
                self.diagnostics.push(
                    Diagnostic::error(DiagnosticKind::ParseError, format!("Parse error: {}", e))
                        .with_span(Span::new(0, sql.len().min(50))),
                );
                return Err(std::mem::take(&mut self.diagnostics));
            }
        };

        for stmt in statements {
            self.process_statement(&stmt);
        }

        if self
            .diagnostics
            .iter()
            .any(|d| d.severity == crate::error::Severity::Error)
        {
            Err(std::mem::take(&mut self.diagnostics))
        } else {
            Ok(())
        }
    }

    /// Process a single SQL statement
    fn process_statement(&mut self, stmt: &Statement) {
        // Currently only handles CREATE TABLE
        // TODO: Handle other statements (CREATE INDEX, ALTER TABLE, etc.)
        if let Statement::CreateTable(create) = stmt {
            self.process_create_table(create);
        }
    }

    /// Process CREATE TABLE statement
    fn process_create_table(&mut self, create: &sqlparser::ast::CreateTable) {
        let name = object_name_to_qualified(&create.name);
        let mut table = TableDef::new(name);

        // Process columns
        for column in &create.columns {
            let col_name = column.name.value.clone();
            let data_type = SqlType::from_ast(&column.data_type);

            let mut col_def = ColumnDef::new(&col_name, data_type);

            // Process column options
            for option in &column.options {
                self.process_column_option(&mut col_def, option);
            }

            table.columns.insert(col_name, col_def);
        }

        // Process table constraints
        for constraint in &create.constraints {
            self.process_table_constraint(&mut table, constraint);
        }

        self.catalog.add_table(table);
    }

    /// Process a column option (NOT NULL, DEFAULT, PRIMARY KEY, etc.)
    fn process_column_option(&mut self, col: &mut ColumnDef, option: &ColumnOptionDef) {
        match &option.option {
            ColumnOption::Null => {
                col.nullable = true;
            }
            ColumnOption::NotNull => {
                col.nullable = false;
            }
            ColumnOption::Default(expr) => {
                col.default = Some(expr_to_default(expr));
            }
            ColumnOption::Unique { is_primary, .. } => {
                if *is_primary {
                    col.is_primary_key = true;
                    col.nullable = false;
                }
            }
            // Handle SERIAL types implicitly
            ColumnOption::Generated { .. } => {
                // Generated columns are typically NOT NULL with a default
            }
            _ => {}
        }
    }

    /// Process a table constraint (PRIMARY KEY, FOREIGN KEY, UNIQUE)
    fn process_table_constraint(&mut self, table: &mut TableDef, constraint: &TableConstraint) {
        match constraint {
            TableConstraint::PrimaryKey { columns, name, .. } => {
                let pk = PrimaryKeyDef {
                    name: name.as_ref().map(|n| n.value.clone()),
                    columns: columns.iter().map(|c| c.value.clone()).collect(),
                };
                // Mark columns as primary key
                for col_name in &pk.columns {
                    if let Some(col) = table.columns.get_mut(col_name) {
                        col.is_primary_key = true;
                        col.nullable = false;
                    }
                }
                table.primary_key = Some(pk);
            }
            TableConstraint::ForeignKey {
                columns,
                foreign_table,
                referred_columns,
                name,
                ..
            } => {
                let fk = ForeignKeyDef {
                    name: name.as_ref().map(|n| n.value.clone()),
                    columns: columns.iter().map(|c| c.value.clone()).collect(),
                    references_table: object_name_to_qualified(foreign_table),
                    references_columns: referred_columns.iter().map(|c| c.value.clone()).collect(),
                };
                table.foreign_keys.push(fk);
            }
            TableConstraint::Unique { columns, name, .. } => {
                let unique = UniqueConstraintDef {
                    name: name.as_ref().map(|n| n.value.clone()),
                    columns: columns.iter().map(|c| c.value.clone()).collect(),
                };
                table.unique_constraints.push(unique);
            }
            _ => {}
        }
    }

    /// Consume the builder and return the catalog
    pub fn build(self) -> (Catalog, Vec<Diagnostic>) {
        (self.catalog, self.diagnostics)
    }

    /// Get a reference to the current catalog
    #[allow(dead_code)]
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert sqlparser ObjectName to our QualifiedName
fn object_name_to_qualified(name: &ObjectName) -> QualifiedName {
    match name.0.as_slice() {
        [table] => QualifiedName::new(&table.value),
        [schema, table] => QualifiedName::with_schema(&schema.value, &table.value),
        [_catalog, schema, table] => QualifiedName::with_schema(&schema.value, &table.value),
        _ => QualifiedName::new(name.to_string()),
    }
}

/// Convert expression to DefaultValue
fn expr_to_default(expr: &sqlparser::ast::Expr) -> DefaultValue {
    match expr {
        sqlparser::ast::Expr::Value(v) => match v {
            sqlparser::ast::Value::Null => DefaultValue::Null,
            _ => DefaultValue::Literal(v.to_string()),
        },
        sqlparser::ast::Expr::Function(f) => {
            let func_name = f.name.to_string().to_lowercase();
            if func_name.contains("now") || func_name.contains("current_timestamp") {
                DefaultValue::CurrentTimestamp
            } else if func_name.contains("nextval") {
                DefaultValue::NextVal(f.to_string())
            } else {
                DefaultValue::Expression(f.to_string())
            }
        }
        _ => DefaultValue::Expression(expr.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_table() {
        let sql = r#"
            CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                email TEXT UNIQUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        "#;

        let mut builder = SchemaBuilder::new();
        builder.parse(sql).unwrap();
        let (catalog, _) = builder.build();

        let table = catalog.get_table(&QualifiedName::new("users")).unwrap();
        assert_eq!(table.columns.len(), 4);

        let id_col = table.get_column("id").unwrap();
        assert!(!id_col.nullable);
        assert!(id_col.is_primary_key);

        let name_col = table.get_column("name").unwrap();
        assert!(!name_col.nullable);
        assert!(matches!(name_col.data_type, SqlType::Varchar { .. }));

        let email_col = table.get_column("email").unwrap();
        assert!(email_col.nullable);
    }

    #[test]
    fn test_parse_table_with_foreign_key() {
        let sql = r#"
            CREATE TABLE orders (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL REFERENCES users(id),
                total DECIMAL(10, 2)
            );
        "#;

        let mut builder = SchemaBuilder::new();
        builder.parse(sql).unwrap();
        let (catalog, _) = builder.build();

        let table = catalog.get_table(&QualifiedName::new("orders")).unwrap();
        assert_eq!(table.columns.len(), 3);
    }
}
