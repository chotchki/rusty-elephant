//! The current goal of the row manager is to provide an interative interface over a table's pages.
//!
//! It will provide a raw scan over a table, insert and update. Update has been deferred until I figure out transactions.
//!
//! NO LOCKING or transaction control yet. I did implement it at the IO layer but its probably wrong.

use super::super::super::constants::BuiltinSqlTypes;
use super::super::objects::{Table, TransactionId};
use super::page_formats::{PageData, PageDataError};
use super::row_formats::{RowData, RowDataError};
use super::{IOManager, IOManagerError};
use async_stream::try_stream;
use futures::stream::Stream;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub struct RowManager {
    io_manager: IOManager,
}

impl RowManager {
    fn new(io_manager: IOManager) -> RowManager {
        RowManager { io_manager }
    }

    pub async fn insert_row(
        &mut self,
        current_tran_id: TransactionId,
        table: Arc<Table>,
        user_data: Vec<Option<BuiltinSqlTypes>>,
    ) -> Result<(), RowManagerError> {
        let row = RowData::new(table.clone(), current_tran_id, None, user_data)?;
        let row_len = row.serialize().len();

        let mut page_num = 0;
        loop {
            let page_bytes = self.io_manager.get_page(table.clone(), page_num).await;
            match page_bytes {
                Some(p) => {
                    let mut page = PageData::parse(table.clone(), p)?;
                    if page.can_fit(row_len) {
                        page.store(row)?;
                        let new_page_bytes = page.serialize();
                        self.io_manager
                            .update_page(table, new_page_bytes, page_num)
                            .await?;
                        return Ok(());
                    } else {
                        page_num += 1;
                        continue;
                    }
                }
                None => {
                    let mut new_page = PageData::new(table.clone());
                    new_page.store(row)?; //TODO Will NOT handle overly large rows
                    self.io_manager.add_page(table, new_page.serialize()).await;
                    return Ok(());
                }
            }
        }
    }

    pub fn get_stream(
        &self,
        table: Arc<Table>,
    ) -> impl Stream<Item = Result<RowData, RowManagerError>> {
        let io_man = self.io_manager.clone();
        try_stream! {
            for await page_bytes in io_man.get_stream(table.clone()) {
                let page = PageData::parse(table.clone(), page_bytes)?;
                for await row in page.get_stream() {
                    yield row;
                }
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum RowManagerError {
    #[error("Page Data Parse Error")]
    PageDataParseError(#[from] PageDataError),
    #[error("IO Manager Error")]
    IOManagerError(#[from] IOManagerError),
    #[error("Row Data Error")]
    RowDataError(#[from] RowDataError),
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::super::super::super::constants::DeserializeTypes;
    use super::super::super::objects::Attribute;
    use super::super::super::objects::Table;
    use super::*;
    use futures::pin_mut;
    use futures::stream::StreamExt;

    //Async testing help can be found here: https://blog.x5ff.xyz/blog/async-tests-tokio-rust/
    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    fn get_table() -> Arc<Table> {
        Arc::new(Table::new(
            "test_table".to_string(),
            vec![
                Attribute::new(
                    uuid::Uuid::new_v4(),
                    "header".to_string(),
                    DeserializeTypes::Text,
                ),
                Attribute::new(
                    uuid::Uuid::new_v4(),
                    "id".to_string(),
                    DeserializeTypes::Uuid,
                ),
                Attribute::new(
                    uuid::Uuid::new_v4(),
                    "header3".to_string(),
                    DeserializeTypes::Text,
                ),
            ],
        ))
    }

    fn get_row() -> Vec<Option<BuiltinSqlTypes>> {
        vec![
                Some(BuiltinSqlTypes::Text("this is a test".to_string())),
                None,
                Some(BuiltinSqlTypes::Text("blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah".to_string())),
            ]
    }

    #[test]
    fn test_row_manager_mass_insert() {
        let table = get_table();
        let pm = IOManager::new();
        let mut rm = RowManager::new(pm);

        let tran_id = TransactionId::new(1);

        for _ in 0..500 {
            assert!(aw!(rm.insert_row(tran_id, table.clone(), get_row())).is_ok());
        }

        //Now let's make sure they're really in the table
        pin_mut!(rm);
        let result_rows: Vec<RowData> =
            aw!(rm.get_stream(table.clone()).map(Result::unwrap).collect());

        let sample_row = get_row();
        for row in result_rows {
            assert_eq!(row.user_data, sample_row);
        }
    }
}
