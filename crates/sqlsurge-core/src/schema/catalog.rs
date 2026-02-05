//! Schema catalog - stores table and column definitions

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::types::SqlType;

/// Schema catalog - holds all table/view information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    /// Schema name -> Schema
    pub schemas: IndexMap<String, Schema>,
    /// Default schema name (e.g., "public" for PostgreSQL)
    pub default_schema: String,
}

impl Catalog {
    pub fn new() -> Self {
        let mut catalog = Self {
            schemas: IndexMap::new(),
            default_schema: "public".to_string(),
        };
        // Create default schema
        catalog.schemas.insert(
            "public".to_string(),
            Schema {
                name: "public".to_string(),
                tables: IndexMap::new(),
            },
        );
        catalog
    }

    /// Get or create a schema
    pub fn get_or_create_schema(&mut self, name: &str) -> &mut Schema {
        if !self.schemas.contains_key(name) {
            self.schemas.insert(
                name.to_string(),
                Schema {
                    name: name.to_string(),
                    tables: IndexMap::new(),
                },
            );
        }
        self.schemas.get_mut(name).unwrap()
    }

    /// Add a table to the catalog
    pub fn add_table(&mut self, table: TableDef) {
        let schema_name = table
            .name
            .schema
            .clone()
            .unwrap_or_else(|| self.default_schema.clone());
        let schema = self.get_or_create_schema(&schema_name);
        schema.tables.insert(table.name.name.clone(), table);
    }

    /// Look up a table by name
    pub fn get_table(&self, name: &QualifiedName) -> Option<&TableDef> {
        let schema_name = name.schema.as_ref().unwrap_or(&self.default_schema);
        self.schemas
            .get(schema_name)
            .and_then(|s| s.tables.get(&name.name))
    }

    /// Check if a table exists
    pub fn table_exists(&self, name: &QualifiedName) -> bool {
        self.get_table(name).is_some()
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<QualifiedName> {
        self.schemas
            .iter()
            .flat_map(|(schema_name, schema)| {
                schema.tables.keys().map(move |table_name| QualifiedName {
                    schema: Some(schema_name.clone()),
                    name: table_name.clone(),
                })
            })
            .collect()
    }
}

/// A database schema (namespace)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub tables: IndexMap<String, TableDef>,
}

/// Qualified name (schema.table or just table)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QualifiedName {
    pub schema: Option<String>,
    pub name: String,
}

impl QualifiedName {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: None,
            name: name.into(),
        }
    }

    pub fn with_schema(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: Some(schema.into()),
            name: name.into(),
        }
    }

    /// Parse from a dotted name like "schema.table" or just "table"
    pub fn parse(s: &str) -> Self {
        if let Some((schema, name)) = s.split_once('.') {
            Self::with_schema(schema, name)
        } else {
            Self::new(s)
        }
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = &self.schema {
            write!(f, "{}.{}", schema, self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDef {
    pub name: QualifiedName,
    pub columns: IndexMap<String, ColumnDef>,
    pub primary_key: Option<PrimaryKeyDef>,
    pub foreign_keys: Vec<ForeignKeyDef>,
    pub unique_constraints: Vec<UniqueConstraintDef>,
}

impl TableDef {
    pub fn new(name: QualifiedName) -> Self {
        Self {
            name,
            columns: IndexMap::new(),
            primary_key: None,
            foreign_keys: Vec::new(),
            unique_constraints: Vec::new(),
        }
    }

    /// Get a column by name
    pub fn get_column(&self, name: &str) -> Option<&ColumnDef> {
        // Case-insensitive lookup
        self.columns
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    /// Check if a column exists
    pub fn column_exists(&self, name: &str) -> bool {
        self.get_column(name).is_some()
    }

    /// Get all column names
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.keys().map(|s| s.as_str()).collect()
    }
}

/// Column definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: SqlType,
    pub nullable: bool,
    pub default: Option<DefaultValue>,
    pub is_primary_key: bool,
}

impl ColumnDef {
    pub fn new(name: impl Into<String>, data_type: SqlType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            default: None,
            is_primary_key: false,
        }
    }

    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    pub fn with_default(mut self, default: DefaultValue) -> Self {
        self.default = Some(default);
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self.nullable = false;
        self
    }
}

/// Default value for a column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefaultValue {
    Literal(String),
    Expression(String),
    CurrentTimestamp,
    Null,
    NextVal(String), // For sequences/SERIAL
}

/// Primary key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryKeyDef {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// Foreign key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyDef {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub references_table: QualifiedName,
    pub references_columns: Vec<String>,
}

/// Unique constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConstraintDef {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualified_name_parse() {
        let name = QualifiedName::parse("users");
        assert_eq!(name.schema, None);
        assert_eq!(name.name, "users");

        let name = QualifiedName::parse("public.users");
        assert_eq!(name.schema, Some("public".to_string()));
        assert_eq!(name.name, "users");
    }

    #[test]
    fn test_catalog_add_table() {
        let mut catalog = Catalog::new();
        let table = TableDef::new(QualifiedName::new("users"));
        catalog.add_table(table);

        assert!(catalog.table_exists(&QualifiedName::new("users")));
        assert!(catalog.table_exists(&QualifiedName::with_schema("public", "users")));
    }
}
