//! SQL dialect support

use sqlparser::dialect::{Dialect, PostgreSqlDialect};
use std::str::FromStr;

/// Supported SQL dialects
#[derive(Debug, Clone, Copy, Default)]
pub enum SqlDialect {
    #[default]
    PostgreSQL,
    // Future: MySQL, SQLite, etc.
}

impl SqlDialect {
    /// Get the sqlparser dialect
    pub fn parser_dialect(&self) -> Box<dyn Dialect> {
        match self {
            SqlDialect::PostgreSQL => Box::new(PostgreSqlDialect {}),
        }
    }

    /// Get default schema name for this dialect
    pub fn default_schema(&self) -> &'static str {
        match self {
            SqlDialect::PostgreSQL => "public",
        }
    }
}

impl FromStr for SqlDialect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgresql" | "postgres" | "pg" => Ok(SqlDialect::PostgreSQL),
            _ => Err(format!("Unknown dialect: {}", s)),
        }
    }
}

impl std::fmt::Display for SqlDialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlDialect::PostgreSQL => write!(f, "postgresql"),
        }
    }
}
