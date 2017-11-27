use r2d2_diesel::ConnectionManager;
use diesel::prelude::*;
use r2d2;

use std::path::PathBuf;
use std::sync::Arc;
use std::io;
use std::time::Duration;

use errors::*;

#[cfg(not(test))]
use xdg_dirs;

type Pool = Arc<r2d2::Pool<ConnectionManager<SqliteConnection>>>;

embed_migrations!("migrations/");

lazy_static!{
    static ref POOL: Pool = init_pool(DB_PATH.to_str().unwrap());
}

#[cfg(not(test))]
lazy_static! {
    static ref DB_PATH: PathBuf = xdg_dirs::HAMMOND_XDG.place_data_file("hammond.db").unwrap();
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

pub(crate) fn connection() -> Pool {
    Arc::clone(&POOL)
}

fn init_pool(db_path: &str) -> Pool {
    let config = r2d2::Config::builder()
        .pool_size(1)
        .connection_timeout(Duration::from_secs(60))
        .build();
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);
    let pool = Arc::new(r2d2::Pool::new(config, manager).expect("Failed to create pool."));

    {
        let db = Arc::clone(&pool).get().expect("Failed to initialize pool.");
        run_migration_on(&*db).expect("Failed to run migrations during init.");
    }
    info!("Database pool initialized.");
    pool
}

fn run_migration_on(connection: &SqliteConnection) -> Result<()> {
    info!("Running DB Migrations...");
    // embedded_migrations::run(connection)?;
    embedded_migrations::run_with_output(connection, &mut io::stdout())?;
    Ok(())
}

// Reset the database into a clean state.
// Test share a Temp file db.
#[allow(dead_code)]
pub fn truncate_db() -> Result<()> {
    let db = connection();
    let con = db.get()?;
    con.execute("DELETE FROM episode")?;
    con.execute("DELETE FROM podcast")?;
    con.execute("DELETE FROM source")?;
    Ok(())
}
