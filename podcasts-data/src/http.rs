// http.rs
//
// Copyright 2026 nee <nee-git@patchouli.garden>
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

// Module with generic helpers for all HTTP requests we do.

use crate::{USER_AGENT_CUSTOM, USER_AGENT_GENERIC};
use url::Url;

use reqwest::RequestBuilder;
use reqwest::redirect::Policy;

/// reqwest already has a retry policy, but we need a custom handler for
/// trying different user agents.
#[derive(Default)]
pub struct RetryContext {
    tries: u8,
    tried_custom_ua: bool,
}

impl RetryContext {
    const MAX_RETRIES: u8 = 2;

    fn should_stop(&self) -> bool {
        self.tries > Self::MAX_RETRIES
    }

    fn client(&self) -> Result<reqwest::Client, reqwest::Error> {
        if self.tried_custom_ua {
            client_builder().user_agent(USER_AGENT_GENERIC).build()
        } else {
            client_builder().user_agent(USER_AGENT_CUSTOM).build()
        }
    }

    fn advance(&mut self) {
        self.tried_custom_ua = true;
        self.tries += 1;
    }

    pub async fn prepared_send<F>(
        &mut self,
        uri: Url,
        prepare: F,
    ) -> Result<reqwest::Response, reqwest::Error>
    where
        F: Fn(RequestBuilder) -> RequestBuilder,
    {
        loop {
            let client = self.client()?;
            self.advance();
            let base_request = client.get(uri.clone());
            let request = prepare(base_request).build()?;
            let result = client.execute(request).await;
            if result.is_ok() || self.should_stop() {
                return result;
            }
        }
    }
}

pub fn client_builder() -> reqwest::ClientBuilder {
    // Haven't included the loop check as
    // Steal the Stars would trigger it as
    // it has a loop back before giving correct url
    let policy = Policy::custom(|attempt| {
        info!("Redirect Attempt URL: {:?}", attempt.url());
        if attempt.previous().len() > 20 {
            attempt.error("too many redirects")
        } else if Some(attempt.url()) == attempt.previous().last() {
            // avoid redirect loops
            attempt.stop()
        } else {
            attempt.follow()
        }
    });

    reqwest::Client::builder()
        .redirect(policy)
        .referer(false)
        // required to keep dead feeds from blocking a refresh for multiple minutes
        .connect_timeout(std::time::Duration::from_secs(20))
}
