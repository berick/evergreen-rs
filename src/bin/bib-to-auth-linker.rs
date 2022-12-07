/*
 * This script was built from the KCLS authority_control_fields.pl
 * script.  It varies from stock Evergreen.  It should be possible to
 * sync with stock Evergreen with additional command line options.
 */
use std::io;
use std::rc::Rc;
use std::time::Instant;
use std::cell::RefCell;
use std::collections::HashMap;
use getopts;
use evergreen as eg;
use opensrf as osrf;
use eg::idl;
use eg::init;
use eg::db::DatabaseConnection;

const DEFAULT_STAFF_ACCOUNT: u32 = 4953211; // utiladmin
const DEFAULT_CONTROL_NUMBER_IDENTIFIER: &str = "DLC";

struct ControlledField {
    bib_field: String,
    bib_subfield: String,
    auth_field: String,
    auth_subfield: String,
}

struct BibLinker {
    ctx: init::Context,
    db: Rc<RefCell<DatabaseConnection>>,
    editor: eg::Editor,
    staff_account: u32,
    start_id: i64,
    end_id: Option<i64>,
}

impl BibLinker {
    fn new(opts: &mut getopts::Options) -> Result<Self, String> {

        let ctx = init::init_with_options(opts)?;
        let editor = eg::Editor::new(ctx.client(), ctx.idl());

        let params = ctx.params();

        let mut db = DatabaseConnection::new_from_options(params);
        db.connect()?;

        let db = db.into_shared();

        let sa = DEFAULT_STAFF_ACCOUNT.to_string();
        let staff_account = params.opt_get_default("staff-account", sa).unwrap();
        let staff_account = match staff_account.parse::<u32>() {
            Ok(id) => id,
            Err(e) => Err(format!("Error parsing staff-account value: {e}"))?,
        };

        Ok(BibLinker {
            ctx,
            db,
            editor,
            staff_account,
            start_id: 1, // TODO
            end_id: None, // TODO
        })
    }

    fn ctx(&self) -> &init::Context {
        &self.ctx
    }

    fn db(&self) -> &Rc<RefCell<DatabaseConnection>> {
        &self.db
    }

    fn get_bib_ids(&self) -> Result<Vec<i64>, String> {

        let select = "SELECT id";
        let from = "FROM biblio.record_entry";

        let mut where_ = format!("WHERE NOT deleted AND id >= {}", self.start_id);
        if let Some(end) = self.end_id {
            where_ += &format!(" AND id < {end}");
        }

        let order = "ORDER BY id";

        let sql = format!("{select} {from} {where_} {order}");

        let query_res = self.db().borrow_mut().client().query(&sql[..], &[]);

        let rows = match query_res {
            Ok(rows) => rows,
            Err(e) => Err(format!("Failed getting bib IDs: {e}"))?,
        };

        let mut list: Vec<i64> = Vec::new();
        for row in rows {
            let id: Option<i64> = row.get("id");
            list.push(id.unwrap());
        }

        Ok(list)
    }

    fn get_controlled_fields(&mut self) -> Result<Vec<ControlledField>, String> {

        let search = json::object!{"id":json::object!{"<>":json::JsonValue::Null}};
        let flesh = json::object! {
            flesh: 1,
            flesh_fields: json::object!{
                acsbf: vec!["authority_field"]
            }
        };

        let bib_fields = self.editor.search_with_ops("acsbf", search, flesh)?;

        println!("bib fields are {:?}", bib_fields);

        Err(format!("TEST"))
    }

    fn link_bibs(&mut self) -> Result<(), String> {

        self.get_controlled_fields()?;

        for rec_id in self.get_bib_ids()? {
            println!("ID IS {rec_id}");
        }

        Ok(())
    }
}

fn main() -> Result<(), String> {

    let mut opts = getopts::Options::new();

    opts.optopt("", "staff-account", "Staff Account ID", "STAFF_ACCOUNT_ID");

    DatabaseConnection::append_options(&mut opts);

    let mut linker = BibLinker::new(&mut opts)?;
    linker.link_bibs()?;

    Ok(())
}



