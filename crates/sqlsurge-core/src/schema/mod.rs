//! Schema management module

mod builder;
mod catalog;

pub use builder::SchemaBuilder;
pub use catalog::{
    Catalog, ColumnDef, DefaultValue, ForeignKeyDef, PrimaryKeyDef, QualifiedName, Schema,
    TableDef, UniqueConstraintDef,
};
