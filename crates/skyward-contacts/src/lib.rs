//! What the phone's own contacts provider keeps: the people it knows and
//! the calls it recorded.
//!
//! The provider keeps its tables in a database of its own, in the part of
//! the data partition that only opens once the owner's credential has been
//! given. The tables are the ones Android's contacts provider has used for
//! years, where a row of `data` holds a value of any kind and the kind is
//! named by the row's `mimetype_id` — the number of which is the provider's
//! own business and changes between releases, so it is never written down
//! here but looked up in the `mimetypes` table instead.
//!
//! The database is read as it lies: it is opened with `immutable=1`, which
//! tells SQLite that nothing will change it, so it needs neither a lock nor
//! the right to write. What that costs is the write-ahead log beside it —
//! the provider writes there first and copies into the database later — so a
//! copy that keeps the log is opened the ordinary way when the first
//! reading comes back empty.

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

/// Where the contacts provider keeps what it knows.
pub const DATABASE: &str = "/data/user/0/com.android.providers.contacts/databases/contacts2.db";

/// Where the provider keeps the calls that were made.
pub const CALLS: &str = "/data/user/0/com.android.providers.contacts/databases/calllog.db";

/// The kind of value a row of `data` holds when it is a telephone number.
const PHONE: &str = "vnd.android.cursor.item/phone_v2";

/// A person the phone knows, with the numbers it has for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    /// The provider's own number for the person.
    pub id: i64,
    /// The name the phone shows, which may be empty.
    pub name: String,
    /// The numbers the phone has for them.
    pub numbers: Vec<String>,
}

/// A call the phone recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The number the other end had, as the phone saw it.
    pub number: String,
    /// The name the provider kept with the number when it recorded the call.
    /// It may be empty, or out of date; the contacts are asked as well.
    pub name: String,
    /// When it was, in milliseconds since the epoch.
    pub at: i64,
    /// How long it lasted, in seconds.
    pub seconds: i64,
    /// What kind of call it was: 1 incoming, 2 outgoing, 3 missed, 4 voicemail.
    pub kind: i64,
}

/// What kind of call a number of the provider's own means.
pub const INCOMING: i64 = 1;
pub const OUTGOING: i64 = 2;
pub const MISSED: i64 = 3;
pub const VOICEMAIL: i64 = 4;

/// The phone's contacts, as its provider keeps them.
pub struct Contacts {
    db: Connection,
}

impl Contacts {
    /// Open the provider's database, as it lies on the phone.
    pub fn open(database: &Path) -> Result<Self> {
        let at = format!("file:{}?immutable=1", database.display());
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
        let db = Connection::open_with_flags(&at, flags).with_context(|| format!("open {}", database.display()))?;
        Ok(Self { db })
    }

    /// Every person the provider knows, in the order the phone would show them.
    pub fn all(&self) -> Result<Vec<Contact>> {
        self.gather(
            "SELECT c._id, name_raw_contact.display_name, d.data1
             FROM contacts c
             JOIN raw_contacts AS name_raw_contact ON (c.name_raw_contact_id = name_raw_contact._id)
             JOIN raw_contacts r ON r.contact_id = c._id
             JOIN data d ON d.raw_contact_id = r._id
             JOIN mimetypes m ON m._id = d.mimetype_id
             WHERE m.mimetype = ?1 AND r.deleted = 0 AND name_raw_contact.deleted = 0 AND d.data1 IS NOT NULL
             ORDER BY c._id",
            None,
        )
    }

    /// The people whose name or one of their numbers holds `text`.
    pub fn search(&self, text: &str) -> Result<Vec<Contact>> {
        let text = text.trim();
        if text.is_empty() {
            return self.all();
        }
        let like = format!("%{text}%");
        self.gather(
            "SELECT c._id, name_raw_contact.display_name, d.data1
             FROM contacts c
             JOIN raw_contacts AS name_raw_contact ON (c.name_raw_contact_id = name_raw_contact._id)
             JOIN raw_contacts r ON r.contact_id = c._id
             JOIN data d ON d.raw_contact_id = r._id
             JOIN mimetypes m ON m._id = d.mimetype_id
             WHERE m.mimetype = ?1 AND r.deleted = 0 AND name_raw_contact.deleted = 0 AND d.data1 IS NOT NULL
               AND (name_raw_contact.display_name LIKE ?2 OR d.data1 LIKE ?2)
             ORDER BY c._id",
            Some(like),
        )
    }

    /// The names the provider holds on its own: the display name each of its
    /// raw rows carries, with the repeats taken out.
    pub fn names(&self) -> Result<Vec<String>> {
        let mut query = self
            .db
            .prepare(
                "SELECT DISTINCT display_name FROM raw_contacts
                 WHERE deleted = 0 AND display_name IS NOT NULL",
            )
            .context("ask the provider for the names it holds")?;
        let mut rows = query.query([]).context("ask the provider for the names it holds")?;
        let mut names = Vec::new();
        while let Some(row) = rows.next().context("read the names the provider holds")? {
            let name: String = row.get(0).context("read a name")?;
            let name = name.trim();
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }

    /// Ask the provider for the people it knows, with the numbers it has
    /// for them. The provider keeps one row per number, so a person with two
    /// numbers shows up twice; the second time they are met, their number is
    /// added to the one already gathered.
    ///
    /// `extra` is bound to the second parameter the statement may name, and
    /// is left out when the statement names none: SQLite insists on a value
    /// for every parameter it is given, and on none it does not name.
    fn gather(&self, sql: &str, extra: Option<String>) -> Result<Vec<Contact>> {
        let mut query = self.db.prepare(sql).context("ask the provider for the people it knows")?;
        let mut rows = match &extra {
            Some(text) => query.query(rusqlite::params![PHONE, text]),
            None => query.query(rusqlite::params![PHONE]),
        }
        .context("ask the provider for the people it knows")?;
        let mut people: Vec<Contact> = Vec::new();
        while let Some(row) = rows.next().context("read the people the provider knows")? {
            let id: i64 = row.get(0).context("read a person's number")?;
            let name: Option<String> = row.get(1).context("read a person's name")?;
            let number: String = row.get(2).context("read a person's number")?;
            let number = number.trim();
            if number.is_empty() {
                continue;
            }
            match people.iter_mut().find(|p| p.id == id) {
                Some(person) => {
                    if !person.numbers.iter().any(|n| n == number) {
                        person.numbers.push(number.to_string());
                    }
                }
                None => people.push(Contact {
                    id,
                    name: name.unwrap_or_default(),
                    numbers: vec![number.to_string()],
                }),
            }
        }
        Ok(people)
    }
}

/// The calls the provider recorded.
pub struct Calls {
    db: Connection,
}

impl Calls {
    /// Open the provider's record of calls, as it lies on the phone.
    pub fn open(database: &Path) -> Result<Self> {
        let at = format!("file:{}?immutable=1", database.display());
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
        let db = Connection::open_with_flags(&at, flags).with_context(|| format!("open {}", database.display()))?;
        Ok(Self { db })
    }

    /// The calls, the most recent first, at most `limit` of them.
    pub fn recent(&self, limit: usize) -> Result<Vec<Call>> {
        let mut query = self
            .db
            .prepare("SELECT number, date, duration, type, name FROM calls WHERE deleted = 0 ORDER BY date DESC LIMIT ?1")
            .context("ask the provider for the calls it recorded")?;
        let mut rows = query.query((limit as i64,)).context("ask the provider for the calls it recorded")?;
        let mut calls = Vec::new();
        while let Some(row) = rows.next().context("read the calls the provider recorded")? {
            let number: Option<String> = row.get(0).context("read a call's number")?;
            let at: i64 = row.get(1).context("read when a call was")?;
            let seconds: i64 = row.get(2).context("read how long a call was")?;
            let kind: i64 = row.get(3).context("read what kind of call it was")?;
            let name: Option<String> = row.get(4).context("read the name kept with a call")?;
            calls.push(Call {
                number: number.unwrap_or_default(),
                name: name.unwrap_or_default().trim().to_string(),
                at,
                seconds,
                kind,
            });
        }
        Ok(calls)
    }
}

/// The provider's package, whose databases live among the owner's own files.
const PACKAGE: &str = "com.android.providers.contacts";

/// Every place one of the provider's databases may lie. A recovery mounts
/// the data partition itself, so the running system's `/data/user/0` view
/// need not be the path that is there: some phones keep the same files under
/// `/data/data` or a credential-encrypted area, and a second space or work
/// profile has a numbered area of its own.
pub fn places(file: &str) -> Vec<PathBuf> {
    fn add(out: &mut Vec<PathBuf>, at: PathBuf) {
        if !out.contains(&at) {
            out.push(at);
        }
    }
    let mut out = Vec::new();
    for root in ["/data/user/0", "/data/data", "/data/user_ce/0", "/data/user_de/0"] {
        add(&mut out, PathBuf::from(format!("{root}/{PACKAGE}/databases/{file}")));
    }
    for parent in ["/data/user", "/data/user_ce", "/data/user_de"] {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                    add(&mut out, entry.path().join(PACKAGE).join("databases").join(file));
                }
            }
        }
    }
    // A phone with file-based encryption may give its credential-encrypted
    // areas a name this recovery does not know beforehand (`user_ce`,
    // `misc_ce`, `system_ce`, ...). Look one level into /data for any area
    // that keeps a user's own files, and into each numbered user in it.
    if let Ok(entries) = std::fs::read_dir("/data") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !(name.contains("user") || name.contains("_ce") || name.contains("_de") || name == "data") {
                continue;
            }
            let root = entry.path();
            add(&mut out, root.join(PACKAGE).join("databases").join(file));
            add(&mut out, root.join("0").join(PACKAGE).join("databases").join(file));
            if let Ok(users) = std::fs::read_dir(&root) {
                for user in users.flatten() {
                    if user.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                        add(&mut out, user.path().join(PACKAGE).join("databases").join(file));
                    }
                }
            }
        }
    }
    out
}

/// A little writable place for a copy of a database. `/skyward` is the
/// recovery's own writable filesystem; the temporary directory is a fallback.
fn copy_area() -> Result<PathBuf> {
    let fallback = std::env::temp_dir().join("skyward-provider");
    for at in [PathBuf::from("/skyward/provider"), fallback] {
        if std::fs::create_dir_all(&at).is_ok() {
            return Ok(at);
        }
    }
    bail!("nowhere writable to copy the provider's database to")
}

/// Open a copy of the contacts database with its log, using SQLite's ordinary
/// rules. `immutable=1` deliberately ignores the write-ahead log, so on a
/// phone whose provider has not checkpointed yet the main file alone holds
/// nothing; this is the other way of reading it.
fn contacts_copy(at: &Path) -> Result<Contacts> {
    let into = copy_area()?;
    for name in ["contacts2.db", "contacts2.db-wal", "contacts2.db-shm"] {
        let _ = std::fs::remove_file(into.join(name));
    }
    let copy = with_log(at, &into).context("copy the contacts database and its log")?;
    let db = Connection::open(&copy).with_context(|| format!("open the copy {}", copy.display()))?;
    Ok(Contacts { db })
}

/// The same for the call log.
fn calls_copy(at: &Path) -> Result<Calls> {
    let into = copy_area()?;
    for name in ["calllog.db", "calllog.db-wal", "calllog.db-shm"] {
        let _ = std::fs::remove_file(into.join(name));
    }
    let copy = with_log(at, &into).context("copy the call log and its own log")?;
    let db = Connection::open(&copy).with_context(|| format!("open the copy {}", copy.display()))?;
    Ok(Calls { db })
}

/// Read the people the provider knows, looking in every place its database
/// may be and falling back to a copy that keeps the write-ahead log. When
/// nothing comes of it the error says what was tried and why, so the page
/// can explain itself instead of showing an unexplained empty list.
pub fn read_contacts() -> Result<Vec<Contact>> {
    let tried = places("contacts2.db");
    let mut found: Vec<PathBuf> = Vec::new();
    let mut said: Vec<String> = Vec::new();
    for at in &tried {
        if !at.is_file() {
            continue;
        }
        found.push(at.clone());
        match Contacts::open(at).and_then(|c| c.all()) {
            Ok(people) if !people.is_empty() => return Ok(people),
            Ok(_) => said.push(format!("{}: 0 人", at.display())),
            Err(e) => said.push(format!("{}: {e:#}", at.display())),
        }
    }
    for at in &found {
        match contacts_copy(at).and_then(|c| c.all()) {
            Ok(people) if !people.is_empty() => return Ok(people),
            Ok(_) => said.push(format!("{} 连日志: 0 人", at.display())),
            Err(e) => said.push(format!("{} 连日志: {e:#}", at.display())),
        }
    }
    if found.is_empty() {
        bail!("没找到 contacts2.db（找过 {} 处：{}）；data 分区可能还没解密",
              tried.len(),
              tried.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("、"))
    }
    bail!("contacts2.db 读不到联系人：{}", said.join("；"))
}

/// Read the most recent calls the provider recorded, the same way.
pub fn read_calls(limit: usize) -> Result<Vec<Call>> {
    let tried = places("calllog.db");
    let mut found: Vec<PathBuf> = Vec::new();
    let mut said: Vec<String> = Vec::new();
    for at in &tried {
        if !at.is_file() {
            continue;
        }
        found.push(at.clone());
        match Calls::open(at).and_then(|c| c.recent(limit)) {
            Ok(calls) if !calls.is_empty() => return Ok(calls),
            Ok(_) => said.push(format!("{}: 0 条", at.display())),
            Err(e) => said.push(format!("{}: {e:#}", at.display())),
        }
    }
    for at in &found {
        match calls_copy(at).and_then(|c| c.recent(limit)) {
            Ok(calls) if !calls.is_empty() => return Ok(calls),
            Ok(_) => said.push(format!("{} 连日志: 0 条", at.display())),
            Err(e) => said.push(format!("{} 连日志: {e:#}", at.display())),
        }
    }
    if found.is_empty() {
        bail!("没找到 calllog.db（找过 {} 处：{}）；data 分区可能还没解密",
              tried.len(),
              tried.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("、"))
    }
    bail!("calllog.db 读不到通话：{}", said.join("；"))
}

/// A copy of the provider's database that keeps the log beside it, so that
/// what the provider has not yet copied into the database is there to be
/// read. The copy is made where it can be written to, for SQLite wants the
/// right to write the lock file that goes with a log.
pub fn with_log(database: &Path, into: &Path) -> Result<PathBuf> {
    let mut copied: Vec<PathBuf> = Vec::new();
    for name in [database.to_path_buf(), side(database, "-wal"), side(database, "-shm")] {
        if !name.is_file() {
            continue;
        }
        let to = into.join(name.file_name().unwrap_or_default());
        std::fs::copy(&name, &to).with_context(|| format!("copy {} to {}", name.display(), to.display()))?;
        copied.push(to);
    }
    if copied.is_empty() {
        bail!("{} is not there to copy", database.display());
    }
    Ok(copied.remove(0))
}

/// The file that lies beside the given one, with the given ending.
fn side(database: &Path, ending: &str) -> PathBuf {
    let mut name = database.as_os_str().to_os_string();
    name.push(ending);
    PathBuf::from(name)
}

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A place of its own for the test to keep its database in. The name of
    /// the test is in it, for the tests run beside one another.
    fn somewhere() -> PathBuf {
        let who = std::thread::current().name().unwrap_or("test").to_string();
        let who: String = who.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
        let dir = std::env::temp_dir().join(format!("skyward-contacts-{}-{who}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("make a place for the test");
        dir
    }

    #[test]
    fn the_people_the_provider_knows_are_gathered() {
        let dir = somewhere();
        let db = dir.join("contacts2.db");
        testing::make(
            &db,
            &[
                ("Zhang San", &["+8613800138000", "+8613900139000"]),
                ("Li Si", &["+8613700137000"]),
                ("Wang Wu", &["+8613600136000"]),
            ],
        );
        let contacts = Contacts::open(&db).expect("open the provider's database");
        let all = contacts.all().expect("ask for the people it knows");
        assert_eq!(all.len(), 3, "every person is found: {all:?}");
        let zhang = all.iter().find(|c| c.name == "Zhang San").expect("Zhang San is there");
        assert_eq!(zhang.numbers.len(), 2, "both of Zhang San's numbers are kept: {zhang:?}");
        assert!(zhang.numbers.contains(&"+8613800138000".to_string()));
        assert!(zhang.numbers.contains(&"+8613900139000".to_string()));
        let li = all.iter().find(|c| c.name == "Li Si").expect("Li Si is there");
        assert_eq!(li.numbers, vec!["+8613700137000".to_string()]);
    }

    #[test]
    fn a_person_marked_deleted_is_not_among_them() {
        let dir = somewhere();
        let db = dir.join("contacts2.db");
        testing::make(&db, &[("Zhang San", &["+8613800138000"])]);
        // The provider keeps a person it has been told to forget, but still
        // waiting for the account that held it to be told as well.
        rusqlite::Connection::open(&db)
            .expect("open the database")
            .execute("UPDATE raw_contacts SET deleted = 1", [])
            .expect("mark the person as deleted");
        let contacts = Contacts::open(&db).expect("open the provider's database");
        let all = contacts.all().expect("ask for the people it knows");
        assert!(all.is_empty(), "a deleted person is not among them: {all:?}");
    }

    #[test]
    fn the_people_are_found_by_name_and_by_number() {
        let dir = somewhere();
        let db = dir.join("contacts2.db");
        testing::make(
            &db,
            &[
                ("Zhang San", &["+8613800138000"]),
                ("Li Si", &["+8613700137000"]),
            ],
        );
        let contacts = Contacts::open(&db).expect("open the provider's database");
        let by_name = contacts.search("Zhang").expect("search by name");
        assert_eq!(by_name.len(), 1, "one person holds the name: {by_name:?}");
        assert_eq!(by_name[0].name, "Zhang San");
        let by_number = contacts.search("1380013").expect("search by number");
        assert_eq!(by_number.len(), 1, "one person holds the number: {by_number:?}");
        assert_eq!(by_number[0].name, "Zhang San");
        let none = contacts.search("nobody here").expect("search for someone who is not there");
        assert!(none.is_empty(), "no one is found: {none:?}");
        let empty = contacts.search("  ").expect("search for nothing");
        assert_eq!(empty.len(), 2, "searching for nothing finds everyone: {empty:?}");
    }

    #[test]
    fn the_calls_the_provider_recorded_are_read() {
        let dir = somewhere();
        let db = dir.join("calllog.db");
        testing::make_calls(
            &db,
            &[
                ("+8613800138000", 1_700_000_000_000, 30, OUTGOING),
                ("+8613700137000", 1_700_000_100_000, 0, MISSED),
            ],
        );
        let calls = Calls::open(&db).expect("open the provider's record of calls");
        let recent = calls.recent(10).expect("ask for the calls it recorded");
        assert_eq!(recent.len(), 2, "both calls are found: {recent:?}");
        assert_eq!(recent[0].number, "+8613700137000", "the newer call comes first");
        assert_eq!(recent[0].kind, MISSED);
        assert_eq!(recent[1].seconds, 30);
    }

    #[test]
    fn the_name_kept_with_a_call_is_read_with_it() {
        let dir = somewhere();
        let db = dir.join("calllog.db");
        testing::make_calls(&db, &[("+8613800138000", 1_700_000_000_000, 30, OUTGOING)]);
        rusqlite::Connection::open(&db)
            .expect("open the database")
            .execute("UPDATE calls SET name = 'Zhang San'", [])
            .expect("name the caller");
        let calls = Calls::open(&db).expect("open the provider's record of calls");
        let recent = calls.recent(10).expect("ask for the calls it recorded");
        assert_eq!(recent[0].name, "Zhang San");
    }

    #[test]
    fn a_copy_keeps_the_log_that_lies_beside_the_database() {
        let dir = somewhere();
        let db = dir.join("contacts2.db");
        testing::make(&db, &[("Zhang San", &["+8613800138000"])]);
        // The provider writes to a log beside the database first, and the
        // log is what a copy that keeps it will have.
        std::fs::write(dir.join("contacts2.db-wal"), b"a log of its own").expect("write a log");
        let into = dir.join("copy");
        std::fs::create_dir_all(&into).expect("make a place for the copy");
        let copied = with_log(&db, &into).expect("copy the database with its log");
        assert!(copied.is_file(), "the database is copied");
        assert!(into.join("contacts2.db-wal").is_file(), "the log is copied as well");
    }

    #[test]
    fn a_number_of_the_providers_own_is_read_as_a_string() {
        // The provider keeps a number in a column of its own, which may hold
        // a number that is not text at all.
        let dir = somewhere();
        let db = dir.join("contacts2.db");
        testing::make(&db, &[("Zhang San", &["+8613800138000"])]);
        rusqlite::Connection::open(&db)
            .expect("open the database")
            .execute("UPDATE data SET data1 = 13800138000 WHERE mimetype_id != 1", [])
            .expect("put a number that is not text in the column");
        let contacts = Contacts::open(&db).expect("open the provider's database");
        let all = contacts.all().expect("ask for the people it knows");
        assert_eq!(all.len(), 1, "the person is still found: {all:?}");
    }
}
