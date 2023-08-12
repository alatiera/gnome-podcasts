// sync/settings.rs
//
// Copyright 2023-2024 nee <nee-git@patchouli.garden>
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

use chrono::{DateTime, Local, TimeZone, Utc};
use diesel::prelude::*;
use std::collections::HashMap;

use crate::database::connection;
use crate::errors::DataError;
use crate::schema::settings_sync;

#[derive(Insertable, Queryable, Identifiable, AsChangeset, PartialEq)]
#[diesel(table_name = settings_sync)]
#[diesel(treat_none_as_null = true)]
#[diesel(primary_key(server))]
#[derive(Debug, Clone)]
/// Stores server address and if sync is active.
/// Password will be stored in the OS keyring.
/// There is always only one row in this table.
pub struct Settings {
    /// domain of the nextcloud sync server.
    pub server: String,
    /// username on the nextcloud sync server.
    pub user: String,
    /// Toggle if syncing should be done.
    pub active: bool,
    /// Last time a full sync was performed with the remote server.
    pub(crate) last_sync: Option<i64>,
}

impl Settings {
    /// Get the current Sync Settings, returns an `Err` if sync wasn't configured.
    /// Also gets the password from the keyring.
    pub async fn fetch() -> Result<(Settings, String), DataError> {
        let settings = Self::fetch_entry()?;
        let password = Settings::fetch_password().await?;
        Ok((settings, password))
    }

    pub(crate) fn fetch_entry() -> Result<Settings, DataError> {
        use crate::schema::settings_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        let settings = settings_sync.get_result::<Settings>(&mut con)?;
        Ok(settings)
    }

    /// Stores the sync settings in the db and the password in the keyring.
    pub async fn store(server_: &str, user_: &str, password: &str) -> Result<(), DataError> {
        Self::store_password(password).await?;
        Self::store_entry(server_, user_)
    }

    pub(crate) fn store_entry(server_: &str, user_: &str) -> Result<(), DataError> {
        use crate::schema::settings_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        let item = Settings {
            server: server_.to_owned(),
            user: user_.to_owned(),
            active: true,
            last_sync: None,
        };

        diesel::insert_into(settings_sync)
            .values(item)
            .on_conflict(server)
            .do_update()
            .set((user.eq(user_), server.eq(server_)))
            .execute(&mut con)?;
        Ok(())
    }

    /// Deletes the server settings from the db and password from the keyring.
    pub async fn remove() -> Result<(), DataError> {
        use crate::schema::settings_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        diesel::delete(settings_sync).execute(&mut con)?;

        let keyring = oo7::Keyring::new()
            .await
            .map_err::<oo7::Error, _>(From::from)?;
        let attributes = HashMap::from([("nextcloud", "password")]);
        // Try update
        let items = keyring.search_items(&attributes).await?;
        for i in items {
            i.delete().await?;
        }

        Ok(())
    }

    /// Update the sync active status in the db.
    pub fn set_active(active_: bool) -> Result<(), DataError> {
        use crate::schema::settings_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        let mut settings = settings_sync.get_result::<Settings>(&mut con)?;
        settings.active = active_;
        settings.save_changes::<Self>(&mut con)?;
        Ok(())
    }

    async fn store_password(password: &str) -> Result<(), DataError> {
        let keyring = oo7::Keyring::new()
            .await
            .map_err::<oo7::Error, _>(From::from)?;
        let attributes = HashMap::from([("nextcloud", "password")]);
        // Try update
        let items = keyring.search_items(&attributes).await?;
        if let Some(item) = items.first() {
            item.set_secret(password.as_bytes()).await?;
            return Ok(());
        }
        // create a new one
        keyring
            .create_item("Nextcloud Password", &attributes, password.as_bytes(), true)
            .await
            .map_err::<oo7::Error, _>(From::from)?;
        debug!("password stored");
        Ok(())
    }

    async fn fetch_password() -> Result<String, DataError> {
        let keyring = oo7::Keyring::new().await?;
        let items = keyring
            .search_items(&HashMap::from([("nextcloud", "password")]))
            .await?;
        if let Some(item) = items.first() {
            let mut secret_bytes = item.secret().await?;
            let secret_vec: Vec<u8> = std::mem::take(&mut secret_bytes);
            let secret_str = String::from_utf8(secret_vec)?;
            Ok(secret_str)
        } else {
            Err(DataError::Bail("No password in keyring".to_owned()))
        }
    }

    /// Returns time the db was successfully synchronized with the server in the Local timezone.
    pub fn last_sync_local(&self) -> Option<DateTime<Local>> {
        let timestamp = self.last_sync?;
        let date_time = Utc.timestamp_opt(timestamp, 0).single()?;
        Some(date_time.with_timezone(&Local))
    }

    /// Returns wether a successful sync was ever performed.
    pub fn did_first_sync(&self) -> bool {
        self.last_sync.is_some()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::database::*;
    use anyhow::Result;

    #[test]
    fn test_episode_deltas() -> Result<()> {
        truncate_db()?;
        assert!(Settings::fetch_entry().is_err());
        Settings::store_entry("127.0.0.1", "test_user")?;
        assert!(Settings::fetch_entry().is_ok());
        assert!(!Settings::fetch_entry()?.did_first_sync());

        assert!(Settings::fetch_entry()?.active);
        Settings::set_active(false)?;
        assert!(!Settings::fetch_entry()?.active);
        Settings::set_active(true)?;
        assert!(Settings::fetch_entry()?.active);
        assert!(Settings::fetch_entry()?.last_sync_local().is_none());
        Ok(())
    }
}
