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

// mapping of authority leader/11 "Subject heading system/thesaurus"
// to the matching bib record indicator
const AUTH_TO_BIB_IND2: &[(&str, &str)] = &[
    ("a", "0"), // Library of Congress Subject Headings (ADULT)
    ("b", "1"), // Library of Congress Subject Headings (JUVENILE)
    ("c", "2"), // Medical Subject Headings
    ("d", "3"), // National Agricultural Library Subject Authority File
    ("n", "4"), // Source not specified
    ("k", "5"), // Canadian Subject Headings
    ("v", "6"), // Répertoire de vedettes-matière
    ("z", "7"), // Source specified in subfield $2 / Other
];

// Produces a new 6XX ind2 value for values found in subfield $2 when the
// original ind2 value is 7 ("Source specified in subfield $2").
const REMAP_BIB_SF2_TO_IND2: &[(&str, &str)] = &[
    ("lcsh", "0"),
    ("mesh", "2"),
    ("nal",  "3"),
    ("rvm",  "6"),
];

#[derive(Debug)]
struct ControlledField {
    bib_tag: String,
    auth_tag: String,
    subfield: String,
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

        let search = json::object! {"id": {"<>": json::JsonValue::Null}};

        let flesh = json::object! {
            flesh: 1,
            flesh_fields: json::object!{
                acsbf: vec!["authority_field"]
            }
        };

        let bib_fields = self.editor.search_with_ops("acsbf", search, flesh)?;

        let linkable_tag_prefixes = vec!["1", "6", "7", "8"];

        // Skip these for non-6XX fields
        let scrub_subfields1 = vec!["v", "x", "y", "z"];

        // Skip these for scrub_tags2 fields
        let scrub_subfields2 = vec!["m", "o", "r", "s"];
        let scrub_tags2 = vec!["130", "600", "610", "630", "700", "710", "730", "830"];

        let mut controlled_fields: Vec<ControlledField> = Vec::new();

        for bib_field in bib_fields {
            let bib_tag = bib_field["tag"].as_str().unwrap();

            if !linkable_tag_prefixes.contains(&&bib_tag[..1]) {
                continue;
            }

            let auth_tag = bib_field["authority_field"]["tag"].as_str().unwrap();

            // Ignore authority 18X fields
            if auth_tag[..2].eq("18") {
                continue;
            }

            let sf_string = bib_field["authority_field"]["sf_list"].as_str().unwrap();
            let mut subfields: Vec<String> = Vec::new();

            for sf in sf_string.split("") {

                if bib_tag[..1].ne("6") && scrub_subfields1.contains(&sf) {
                    continue;
                }

                if scrub_tags2.contains(&bib_tag) && scrub_subfields2.contains(&sf) {
                    continue;
                }

                subfields.push(sf.to_string());
            }

            for sf in subfields {
                controlled_fields.push(
                    ControlledField {
                        bib_tag: bib_tag.to_string(),
                        auth_tag: auth_tag.to_string(),
                        subfield: sf.to_string()
                    }
                );
            }
        }

        Ok(controlled_fields)
    }

    fn link_bibs(&mut self) -> Result<(), String> {

        let control_fields = self.get_controlled_fields()?;

        for rec_id in self.get_bib_ids()? {
            println!("ID IS {rec_id}");
        }

        println!("FIELDS: {:?}", control_fields);

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



