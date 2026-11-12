//! A provider's database of its own, made for the tests to read: the tables
//! Android's contacts provider keeps, with a few people in them. The shape
//! here is the one the provider actually uses on the device — the display
//! name lives on `raw_contacts`, and `contacts` points at it through
//! `name_raw_contact_id` — so the production reading is exercised against the
//! same join it will meet on the phone.

use rusqlite::Connection;
use std::path::Path;

/// Make a database of the provider's own shape, holding the people given.
/// The first is the name, the rest are their numbers.
pub fn make(path: &Path, people: &[(&str, &[&str])]) {
    let db = Connection::open(path).expect("make the provider's database");
    db.execute_batch(
        "CREATE TABLE mimetypes (_id INTEGER PRIMARY KEY, mimetype TEXT);
         CREATE TABLE accounts (_id INTEGER PRIMARY KEY, account_name TEXT, account_type TEXT);
         CREATE TABLE raw_contacts (
             _id INTEGER PRIMARY KEY AUTOINCREMENT,
             account_id INTEGER,
             contact_id INTEGER,
             deleted INTEGER NOT NULL DEFAULT 0,
             display_name TEXT,
             account_name TEXT,
             account_type INTEGER);
         CREATE TABLE contacts (
             _id INTEGER PRIMARY KEY AUTOINCREMENT,
             name_raw_contact_id INTEGER,
             photo_id INTEGER,
             photo_file_id INTEGER,
             starred INTEGER NOT NULL DEFAULT 0,
             times_contacted INTEGER NOT NULL DEFAULT 0,
             last_time_contacted INTEGER NOT NULL DEFAULT 0,
             contact_last_updated_timestamp INTEGER NOT NULL DEFAULT 0,
             deleted INTEGER NOT NULL DEFAULT 0);
         CREATE TABLE data (
             _id INTEGER PRIMARY KEY AUTOINCREMENT,
             package_id INTEGER,
             mimetype_id INTEGER NOT NULL,
             raw_contact_id INTEGER NOT NULL,
             is_primary INTEGER NOT NULL DEFAULT 0,
             is_super_primary INTEGER NOT NULL DEFAULT 0,
             data_version INTEGER NOT NULL DEFAULT 0,
             data1 TEXT, data2 TEXT, data3 TEXT, data4 TEXT, data5 TEXT,
             data6 TEXT, data7 TEXT, data8 TEXT, data9 TEXT, data10 TEXT,
             data11 TEXT, data12 TEXT, data13 TEXT, data14 TEXT, data15 TEXT);
         CREATE TABLE phone_lookup (
             _id INTEGER PRIMARY KEY AUTOINCREMENT,
             data1 TEXT, data2 TEXT, data3 TEXT,
             display_name TEXT, photo_id INTEGER, photo_file_id INTEGER,
             lookup_key TEXT, sort_key TEXT);",
    )
    .expect("make the provider's tables");

    // The provider's own numbering of the kinds of value: the names of the
    // kinds are what a row means, the numbers are its own business.
    db.execute("INSERT INTO mimetypes (mimetype) VALUES ('vnd.android.cursor.item/phone_v2')", [])
        .expect("number the kind of a telephone number");
    let phone = db.last_insert_rowid();
    db.execute("INSERT INTO mimetypes (mimetype) VALUES ('vnd.android.cursor.item/name')", [])
        .expect("number the kind of a name");
    db.execute("INSERT INTO accounts (account_name, account_type) VALUES ('local', 1)", [])
        .expect("make the account the people belong to");

    for (who, numbers) in people {
        db.execute(
            "INSERT INTO contacts (contact_last_updated_timestamp) VALUES (1)",
            [],
        )
        .expect("make a person");
        let contact = db.last_insert_rowid();
        db.execute(
            "INSERT INTO raw_contacts (account_id, contact_id, account_name, account_type, display_name)
             VALUES (1, ?1, 'local', 1, ?2)",
            (contact, who),
        )
        .expect("make the person's own row");
        let raw = db.last_insert_rowid();
        db.execute(
            "UPDATE contacts SET name_raw_contact_id = ?1 WHERE _id = ?2",
            (raw, contact),
        )
        .expect("point the contact at the row that names it");
        for number in *numbers {
            db.execute(
                "INSERT INTO data (mimetype_id, raw_contact_id, is_super_primary, data1) VALUES (?1, ?2, 1, ?3)",
                (phone, raw, number),
            )
            .expect("give the person a number");
            db.execute(
                "INSERT INTO phone_lookup (data1, display_name, lookup_key) VALUES (?1, ?2, ?3)",
                (number, who, who),
            )
            .expect("index the number");
        }
    }
}

/// Make the provider's record of calls, holding the calls given.
pub fn make_calls(path: &Path, calls: &[(&str, i64, i64, i64)]) {
    let db = Connection::open(path).expect("make the provider's record of calls");
    db.execute_batch(
        "CREATE TABLE calls (
             _id INTEGER PRIMARY KEY AUTOINCREMENT,
             number TEXT,
             date INTEGER,
             duration INTEGER,
             type INTEGER,
             deleted INTEGER NOT NULL DEFAULT 0,
             features INTEGER,
             subscription_id INTEGER);",
    )
    .expect("make the table of calls");
    for (number, at, seconds, kind) in calls {
        db.execute(
            "INSERT INTO calls (number, date, duration, type) VALUES (?1, ?2, ?3, ?4)",
            (number, at, seconds, kind),
        )
        .expect("record a call");
    }
}
