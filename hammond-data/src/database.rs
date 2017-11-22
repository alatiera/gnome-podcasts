// use r2d2_diesel::ConnectionManager;
// use diesel::SqliteConnection;
use diesel::prelude::*;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::io;
// use std::time::Duration;

use xdg_;
use errors::*;

// type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
type Database = Arc<Mutex<SqliteConnection>>;

embed_migrations!("migrations/");

lazy_static!{
    // static ref POOL: Pool = init_pool(DB_PATH.to_str().unwrap());

    static ref DB: Arc<Mutex<SqliteConnection>> = Arc::new(Mutex::new(establish_connection()));
}

#[cfg(not(test))]
lazy_static! {
    static ref DB_PATH: PathBuf = xdg_::HAMMOND_XDG.place_data_file("hammond.db").unwrap();
}

#[cfg(test)]
extern crate tempdir;

#[cfg(test)]
lazy_static! {
    static ref TEMPDIR: tempdir::TempDir = {
        tempdir::TempDir::new("hammond_unit_test").unwrap()
    };

    static ref DB_PATH: PathBuf = TEMPDIR.path().join("hammond.db");
}

pub fn connection() -> Database {
    // POOL.clone()
    Arc::clone(&DB)
}

// fn init_pool(db_path: &str) -> Pool {
//     let config = r2d2::Config::builder()
//         // .pool_size(60)
//         // .min_idle(Some(60))
//         // .connection_timeout(Duration::from_secs(60))
//         .build();
//     let manager = ConnectionManager::<SqliteConnection>::new(db_path);
//     let pool = r2d2::Pool::new(config, manager).expect("Failed to create pool.");
//     info!("Database pool initialized.");

//     {
//         let db = pool.clone().get().unwrap();
//         utils::run_migration_on(&*db).unwrap();
//     }

//     pool
// }

pub fn establish_connection() -> SqliteConnection {
    let database_url = DB_PATH.to_str().unwrap();
    let db = SqliteConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url));
    run_migration_on(&db).unwrap();
    db
}

pub fn run_migration_on(connection: &SqliteConnection) -> Result<()> {
    info!("Running DB Migrations...");
    // embedded_migrations::run(connection)?;
    embedded_migrations::run_with_output(connection, &mut io::stdout())?;
    Ok(())
}
