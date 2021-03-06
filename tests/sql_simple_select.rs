mod common;

use feophantlib::{
    constants::BuiltinSqlTypes,
    engine::objects::{QueryResult, SqlTuple},
};

#[test]
fn simple_select() -> Result<(), Box<dyn std::error::Error>> {
    let (mut tm, mut engine) = common::_create_engine();

    let create_test =
        "create table foo (bar text, baz text not null, another text null)".to_string();

    let tran = aw!(tm.start_trans())?;
    aw!(engine.process_query(tran, create_test))?;
    aw!(tm.commit_trans(tran))?;

    let insert_test =
        "insert into foo (another, baz, bar) values('one', 'two', 'three')".to_string();
    let tran = aw!(tm.start_trans())?;
    aw!(engine.process_query(tran, insert_test))?;
    aw!(tm.commit_trans(tran))?;

    let select_test = "select baz, bar, another from foo;".to_string();
    let tran = aw!(tm.start_trans())?;
    let result = aw!(engine.process_query(tran, select_test));
    let result = match result {
        Ok(o) => o,
        Err(e) => {
            println!("{} {:?}", e, e);
            panic!("Ah crap");
        }
    };

    let select_columns = vec!["baz".to_string(), "bar".to_string(), "another".to_string()];

    let select_row = vec![SqlTuple(vec![
        Some(BuiltinSqlTypes::Text("two".to_string())),
        Some(BuiltinSqlTypes::Text("three".to_string())),
        Some(BuiltinSqlTypes::Text("one".to_string())),
    ])];

    assert_eq!(
        result,
        QueryResult {
            columns: select_columns,
            rows: select_row
        }
    );

    aw!(tm.commit_trans(tran))?;

    Ok(())
}
