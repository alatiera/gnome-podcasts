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

use std::path::PathBuf;
use std::sync::LazyLock;

use crate::errors::DataError;

#[cfg(not(test))]
use crate::xdg_dirs;

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

static POOL: LazyLock<Pool> = LazyLock::new(|| init_pool(DB_PATH.to_str().unwrap()));

#[cfg(not(test))]
static DB_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    xdg_dirs::PODCASTS_XDG
        .place_data_file("podcasts.db")
        .unwrap()
});

#[cfg(test)]
pub(crate) static TEMPDIR: LazyLock<tempfile::TempDir> =
    LazyLock::new(|| tempfile::TempDir::with_prefix("podcasts_unit_test").unwrap());

#[cfg(test)]
static DB_PATH: LazyLock<PathBuf> = LazyLock::new(|| TEMPDIR.path().join("podcasts.db"));

/// Get an r2d2 `SqliteConnection`.
pub(crate) fn connection() -> Pool {
    POOL.clone()
}

fn init_pool(db_path: &str) -> Pool {
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);
    let pool = r2d2::Pool::builder()
        .max_size(1)
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
pub fn truncate_db() -> Result<(), DataError> {
    use diesel::connection::SimpleConnection;
    let db = connection();
    let mut con = db.get()?;
    con.batch_execute("DELETE FROM episodes; DELETE FROM shows; DELETE FROM source")?;
    Ok(())
}
