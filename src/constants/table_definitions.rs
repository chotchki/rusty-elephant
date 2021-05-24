//! This defines all the system internal tables so we can bootstrap the system

use hex_literal::hex;
use uuid::Uuid;

use super::super::engine::objects::PgTable;

#[derive(Copy,Clone)]
pub enum TableDefinitions {
    PgClass, //Tables
}

impl TableDefinitions {
    pub const values: [TableDefinitions; 1] = [TableDefinitions::PgClass];
    pub fn value(self) -> PgTable {
        use TableDefinitions::*;
        match self {
            PgClass => {
                PgTable::new(Uuid::from_bytes(hex!("EE919E33D9054F4889537EBB6CC911EB")), "pg_class".to_string(), Vec::new())
            }
        }
    }
}