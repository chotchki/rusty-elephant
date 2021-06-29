//! The current goal of the row manager is to provide an interative interface over a table's pages.
//!
//! It will provide a raw scan over a table, insert and update. Update has been deferred until I figure out transactions.
//!
//! NO LOCKING or transaction control yet. I did implement it at the IO layer but its probably wrong.

use super::super::super::constants::BuiltinSqlTypes;
use super::super::objects::Table;
use super::super::transactions::{
    TransactionId, TransactionManager, TransactionManagerError, TransactionStatus,
};
use super::page_formats::{PageData, PageDataError, UInt12};
use super::row_formats::{ItemPointer, RowData, RowDataError};
use super::{IOManager, IOManagerError};
use async_stream::try_stream;
use futures::stream::Stream;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct RowManager {
    io_manager: Arc<RwLock<IOManager>>,
}

impl RowManager {
    pub fn new(io_manager: Arc<RwLock<IOManager>>) -> RowManager {
        RowManager { io_manager }
    }

    pub async fn insert_row(
        self,
        current_tran_id: TransactionId,
        table: Arc<Table>,
        user_data: Vec<Option<BuiltinSqlTypes>>,
    ) -> Result<ItemPointer, RowManagerError> {
        let io_mut = self.io_manager.write().await;
        RowManager::insert_row_internal(&io_mut, current_tran_id, table, user_data).await
    }

    //Note this is a logical delete
    //TODO debating if this should respect the visibility map, probably yes just trying to limit the pain
    pub async fn delete_row(
        self,
        current_tran_id: TransactionId,
        table: Arc<Table>,
        row_pointer: ItemPointer,
    ) -> Result<(), RowManagerError> {
        let (mut page, mut row) = self.get(table.clone(), row_pointer).await?;

        if row.max.is_some() {
            return Err(RowManagerError::AlreadyDeleted(
                row_pointer.count,
                row.max.unwrap(),
            ));
        }

        row.max = Some(current_tran_id);

        page.update(row, row_pointer.count)?;

        let io_mut = self.io_manager.write().await;
        io_mut
            .update_page(table, page.serialize(), row_pointer.page)
            .await?;
        Ok(())
    }

    //Note this is an insert new row, delete old row operation
    pub async fn update_row(
        &mut self,
        current_tran_id: TransactionId,
        table: Arc<Table>,
        row_pointer: ItemPointer,
        new_user_data: Vec<Option<BuiltinSqlTypes>>,
    ) -> Result<(), RowManagerError> {
        //First get the current row so we have it for the update/delete
        let (mut old_page, mut old_row) = self.get(table.clone(), row_pointer).await?;

        if old_row.max.is_some() {
            return Err(RowManagerError::AlreadyDeleted(
                row_pointer.count,
                old_row.max.unwrap(),
            ));
        }

        //Serialize with a dummy pointer so we can evaluate space needed for the new row
        let new_row = RowData::new(
            table.clone(),
            current_tran_id,
            None,
            ItemPointer::new(0, UInt12::new(0).unwrap()),
            new_user_data.clone(),
        )?;
        let new_row_len = new_row.serialize().len();

        let io_mut = self.io_manager.write().await;

        //Prefer using the old page if possible
        let new_row_pointer;
        if old_page.can_fit(new_row_len) {
            new_row_pointer = old_page.insert(new_row)?;
        } else {
            new_row_pointer = RowManager::insert_row_internal(
                &io_mut,
                current_tran_id,
                table.clone(),
                new_user_data,
            )
            .await?;
        }

        old_row.max = Some(current_tran_id);
        old_row.item_pointer = new_row_pointer;

        old_page.update(old_row, row_pointer.count)?;

        io_mut
            .update_page(table, old_page.serialize(), row_pointer.page)
            .await?;

        return Ok(());
    }

    pub async fn get(
        &self,
        table: Arc<Table>,
        row_pointer: ItemPointer,
    ) -> Result<(PageData, RowData), RowManagerError> {
        let io = self.io_manager.read().await;
        let page_bytes = io
            .get_page(table.clone(), row_pointer.page)
            .await
            .ok_or_else(|| RowManagerError::NonExistentPage(row_pointer.page))?;
        let page = PageData::parse(table.clone(), row_pointer.page, page_bytes)?;

        let row = page
            .get_row(row_pointer.count)
            .ok_or_else(|| RowManagerError::NonExistentRow(row_pointer.count, row_pointer.page))?
            .clone();

        Ok((page, row))
    }

    // Provides an unfiltered view of the underlying table
    pub fn get_stream(
        self,
        table: Arc<Table>,
    ) -> impl Stream<Item = Result<RowData, RowManagerError>> {
        try_stream! {
            let io_man = self.io_manager.read().await;
            let mut page_num = 0;
            for await page_bytes in io_man.get_stream(table.clone()) {
                let page = PageData::parse(table.clone(), page_num, page_bytes)?;
                for await row in page.get_stream() {
                    yield row;
                }
                page_num += 1;
            }
        }
    }

    async fn insert_row_internal(
        io_manager: &IOManager,
        current_tran_id: TransactionId,
        table: Arc<Table>,
        user_data: Vec<Option<BuiltinSqlTypes>>,
    ) -> Result<ItemPointer, RowManagerError> {
        //Serialize with a dummy pointer so we can evaluate space needed
        let row = RowData::new(
            table.clone(),
            current_tran_id,
            None,
            ItemPointer::new(0, UInt12::new(0).unwrap()),
            user_data,
        )?;
        let row_len = row.serialize().len();

        let mut page_num = 0;
        loop {
            let page_bytes = io_manager.get_page(table.clone(), page_num).await;
            match page_bytes {
                Some(p) => {
                    let mut page = PageData::parse(table.clone(), page_num, p)?;
                    if page.can_fit(row_len) {
                        let new_row_pointer = page.insert(row)?;
                        let new_page_bytes = page.serialize();
                        io_manager
                            .update_page(table, new_page_bytes, page_num)
                            .await?;
                        return Ok(new_row_pointer);
                    } else {
                        page_num += 1;
                        continue;
                    }
                }
                None => {
                    let mut new_page = PageData::new(table.clone(), page_num);
                    let new_row_pointer = new_page.insert(row)?; //TODO Will NOT handle overly large rows
                    io_manager.add_page(table, new_page.serialize()).await;
                    return Ok(new_row_pointer);
                }
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum RowManagerError {
    #[error(transparent)]
    PageDataError(#[from] PageDataError),
    #[error(transparent)]
    IOManagerError(#[from] IOManagerError),
    #[error(transparent)]
    RowDataError(#[from] RowDataError),
    #[error("Page {0} does not exist")]
    NonExistentPage(usize),
    #[error("Row {0} in Page {1} does not exist")]
    NonExistentRow(UInt12, usize),
    #[error("Row {0} already deleted in {1}")]
    AlreadyDeleted(UInt12, TransactionId),
    #[error("Row {0} is not visible")]
    NotVisibleRow(RowData),
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

    fn get_row(input: String) -> Vec<Option<BuiltinSqlTypes>> {
        vec![
                Some(BuiltinSqlTypes::Text(input)),
                None,
                Some(BuiltinSqlTypes::Text("blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah".to_string())),
            ]
    }

    #[test]
    fn test_row_manager_mass_insert() {
        let table = get_table();
        let mut tm = TransactionManager::new();
        let pm = Arc::new(RwLock::new(IOManager::new()));
        let rm = RowManager::new(pm);

        let tran_id = TransactionId::new(1);

        for _ in 0..500 {
            assert!(aw!(rm.clone().insert_row(
                tran_id,
                table.clone(),
                get_row("test".to_string())
            ))
            .is_ok());
        }

        //Now let's make sure they're really in the table
        pin_mut!(rm);
        let result_rows: Vec<RowData> = aw!(rm
            .clone()
            .get_stream(table.clone())
            .map(Result::unwrap)
            .collect());

        let sample_row = get_row("test".to_string());
        for row in result_rows {
            assert_eq!(row.user_data, sample_row);
        }
    }

    #[test]
    fn test_row_manager_crud() {
        let table = get_table();
        let mut tm = TransactionManager::new();
        let pm = Arc::new(RwLock::new(IOManager::new()));
        let rm = RowManager::new(pm);

        let tran_id = TransactionId::new(1);

        let insert_pointer =
            aw!(rm
                .clone()
                .insert_row(tran_id, table.clone(), get_row("test".to_string())));
        assert!(insert_pointer.is_ok());

        let insert_pointer = insert_pointer.unwrap();

        let tran_id_2 = TransactionId::new(2);

        let delete_res = aw!(rm
            .clone()
            .delete_row(tran_id_2, table.clone(), insert_pointer));
        assert!(delete_res.is_ok());
    }
}
