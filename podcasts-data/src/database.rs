// database.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Database Setup. This is only public to help with some unit tests.
// Diesel embed_migrations! triggers the lint

use diesel::prelude::*;
use diesel::r2d2;
use diesel::r2d2::ConnectionManager;

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use std::sync::LazyLock;

#[cfg(not(test))]
use crate::xdg_dirs;
#[cfg(test)]
use std::sync::Mutex;

use crate::errors::DataError;

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

// When using nextest this will only run once, and we
// could initialize a cfg(test) db here.
// However with cargo test, this is shared between each
// [test] and instead we need to reset_db() at
// the begging of each test.
// This is why we have reset_db as opposed to initialize_test_db
#[cfg(not(test))]
static POOL: LazyLock<Pool> = LazyLock::new(|| {
    let pathbuf = xdg_dirs::PODCASTS_XDG
        .place_data_file("podcasts.db")
        .unwrap();
    let db_path = pathbuf.to_str().unwrap();
    init_pool(db_path)
});

// Every test should have reset_db at the top
// As-is this initializes a db and then removes the file
// causing every test that will not have a reset_db to
// fail cause the file no longer exists.
#[cfg(test)]
static POOL: LazyLock<Mutex<Pool>> = LazyLock::new(|| {
    let db = tempfile::Builder::new().tempfile().unwrap();
    let db_path = db.path().to_str().unwrap();
    Mutex::new(init_pool(db_path))
});

/// Get an r2d2 `SqliteConnection`.
#[cfg(not(test))]
pub(crate) fn connection() -> Pool {
    POOL.clone()
}

#[cfg(test)]
pub(crate) fn connection() -> Pool {
    POOL.lock().unwrap().clone()
}

fn init_pool(db_path: &str) -> Pool {
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    {
        let mut db = pool.get().expect("Failed to initialize pool.");
        run_migration_on(&mut db).expect("Failed to run migrations during init.");
    }
    info!("Database pool initialized.");
    pool
}

fn run_migration_on(
    conn: &mut SqliteConnection,
) -> Result<Vec<diesel::migration::MigrationVersion<'_>>, DataError> {
    info!("Running DB Migrations...");
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|_| DataError::DieselMigrationError)
}

/// Reset the database into a clean state.
// Test share a Temp file db.
// This is a test-only function but it's public cause we use it
// in the podcast-gtk tests as well and thus need to export it
#[cfg(test)]
pub fn reset_db() -> Result<tempfile::NamedTempFile, DataError> {
    let db = tempfile::Builder::new()
        .suffix("-podcasts.db")
        .tempfile()
        .unwrap();
    let db_path = db.path().to_str().unwrap();

    let pool = init_pool(db_path);
    let mut lock = POOL.lock().unwrap();
    *lock = pool;
    drop(lock);

    Ok(db)
}

#[cfg(not(test))]
pub fn reset_db() -> Result<tempfile::NamedTempFile, DataError> {
    panic!("Should not be called outside of #[test] cases");
}
