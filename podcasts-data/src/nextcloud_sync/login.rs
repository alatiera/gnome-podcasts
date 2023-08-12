// nextcloud_sync/login.rs
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

use crate::nextcloud_sync::data::{client_builder, parse_url_without_scheme};

use anyhow::{bail, Result};
use reqwest;
use reqwest::Url;
use serde::Deserialize;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct LoginFlowV2 {
    poll: LoginFlowV2Poll,
    login: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct LoginFlowV2Poll {
    token: String,
    endpoint: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct LoginFlowV2Success {
    server: String,
    loginName: String,
    appPassword: String,
}

/// Will get a token from the nextcloud server, then run the `open_browser_callback` fn.
/// Then it will make requests for an app_password from the server with a timeout of 20min.
/// The user will have to login via the browser.
/// Once the user has logged in the Server will respond with an app_password.
///
/// Returns (server, user, password) that can be passed to [crate::sync::Settings::store]
pub async fn launch_browser_login_flow_v2<F>(
    server: &str,
    open_browser_callback: F,
) -> Result<(String, String, String)>
where
    F: FnOnce(&str) -> Result<()> + 'static,
{
    let url = parse_url_without_scheme(server)?.join("/index.php/login/v2")?;
    let resp = client_builder().build()?.post(url).send().await?;
    let json = resp.json::<LoginFlowV2>().await?;
    open_browser_callback(&json.login)?;

    // The token will be valid for 20 minutes.
    // This will return a 404 until authentication is done.
    // Once a 200 is returned it is another json object

    let poll_url = Url::parse(&json.poll.endpoint)?;
    let token_form = &HashMap::from([("token", json.poll.token)]);
    let start_of_polling = std::time::Instant::now();
    while start_of_polling.elapsed().as_secs() <= 20 * 60 {
        let resp = client_builder()
            .build()?
            .post(poll_url.clone())
            .form(token_form)
            .send()
            .await;

        if let Ok(resp) = resp {
            match resp.status().as_u16() {
                200 => {
                    let success = resp.json::<LoginFlowV2Success>().await;
                    match success {
                        Ok(success) => {
                            return Ok((success.server, success.loginName, success.appPassword))
                        }
                        Err(e) => {
                            bail!("Failed to parse nextcloud login flow v2 code 200 response {e}")
                        }
                    }
                }
                404 => continue,
                other => {
                    error!("Unexpected response code during nextcloud login flow v2 {other}")
                }
            }
        }
    }
    bail!("nextcloud login flow v2 failed, by expiring the 20min limit")
}

/// Requests the server to turn the `real_password` into an app_password.
/// If the real_password is already an app password a clone of it will be returned.
/// <https://docs.nextcloud.com/server/latest/developer_manual/client_apis/LoginFlow/index.html#converting-to-app-passwords>
pub async fn retrive_app_password(server: &str, user: &str, real_password: &str) -> Result<String> {
    // curl -u username:password -H 'OCS-APIRequest: true' https://cloud.example.com/ocs/v2.php/core/getapppassword
    let url = parse_url_without_scheme(server)?.join("/ocs/v2.php/core/getapppassword")?;
    let resp = client_builder()
        .build()?
        .get(url)
        .header("OCS-APIRequest", "true")
        .basic_auth(user, Some(real_password))
        .send()
        .await?;

    // 403 means it's already an app_password
    if resp.status() == 403 {
        Ok(real_password.to_owned())
    } else {
        parse_app_password_xml(&resp.text().await?)
    }
}

// <?xml version="1.0"?>
// <ocs>
//         <meta>
//                 <status>ok</status>
//                 <statuscode>200</statuscode>
//                 <message>OK</message>
//         </meta>
//         <data>
//                 <apppassword>M1DqHwuZWwjEC3ku7gJsspR7bZXopwf01kj0XGppYVzEkGtbZBRaXlOUxFZdbgJ6Zk9OwG9x</apppassword>
//         </data>
// </ocs>
fn parse_app_password_xml(body: &str) -> Result<String> {
    use xml::reader::{EventReader, XmlEvent};
    let parser = EventReader::new(body.as_bytes());
    let mut in_apppassword = false;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "apppassword" {
                    in_apppassword = true;
                }
            }
            Ok(XmlEvent::EndElement { .. }) => {
                in_apppassword = false;
            }
            Ok(XmlEvent::Characters(s)) => {
                if in_apppassword {
                    return Ok(s);
                }
            }
            Err(e) => {
                error!("Xml Error: {e}");
                continue;
            }
            _ => (),
        }
    }
    bail!("No apppassword found")
}

/// Deletes the app_password from the nextcloud server.
/// Returns `true` if it succeeds, `false` if not.
/// The nextcloud doc recommends: Even if this fails the account should still be removed locally.
/// For that reason the return value is not a Result.
/// https://docs.nextcloud.com/server/latest/developer_manual/client_apis/LoginFlow/index.html#deleting-an-app-password
pub async fn logout(server: &str, user: &str, app_password: &str) -> bool {
    // curl -u username:app-password -X DELETE -H 'OCS-APIREQUEST: true'  http://localhost/ocs/v2.php/core/apppassword
    if let Err(e) = async {
        let url = parse_url_without_scheme(server)?.join("/ocs/v2.php/core/apppassword")?;
        let resp = client_builder()
            .build()?
            .delete(url)
            .header("OCS-APIRequest", "true")
            .basic_auth(user, Some(app_password))
            .send()
            .await?;

        if resp.status() == 200 {
            Ok(())
        } else {
            bail!("nextcloud: failed to remove app password during logout.");
        }
    }
    .await
    {
        error!("{e}");
        false
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_test_server::http::{Method, Status};
    use http_test_server::TestServer;

    #[test]
    fn test_parse_apppassword_xml() -> Result<()> {
        let s = "<?xml version=\"1.0\"?>
<ocs>
        <meta>
                <status>ok</status>
                <statuscode>200</statuscode>
                <message>OK</message>
        </meta>
        <data>
                <apppassword>M1DqHwuZWwjEC3ku7gJsspR7bZXopwf01kj0XGppYVzEkGtbZBRaXlOUxFZdbgJ6Zk9OwG9x</apppassword>
        </data>
</ocs>";

        let result = parse_app_password_xml(s)?;
        assert_eq!(
            result,
            "M1DqHwuZWwjEC3ku7gJsspR7bZXopwf01kj0XGppYVzEkGtbZBRaXlOUxFZdbgJ6Zk9OwG9x"
        );
        Ok(())
    }

    #[test]
    fn test_parse_apppassword_xml_fail() -> Result<()> {
        let s = "<?xml version=\"1.0\"?>
<nope>no app password here</nope>";
        let result = parse_app_password_xml(s);
        assert!(result.is_err());
        let s = "not xml";
        let result = parse_app_password_xml(s);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_login_flow() -> Result<()> {
        let server = mock_login_nextcloud_server()?;
        let address = format!("http://127.0.0.1:{}", server.port());

        let rt = tokio::runtime::Runtime::new()?;
        let (server, user, password) =
            rt.block_on(launch_browser_login_flow_v2(&address, |_| Ok(())))?;
        assert_eq!("http://127.0.0.1", server);
        assert_eq!("test_user", user);
        assert_eq!("test_app_password", password);
        Ok(())
    }

    #[test]
    fn test_app_password_server() -> Result<()> {
        let server = mock_app_password_nextcloud_server()?;
        let address = format!("http://127.0.0.1:{}", server.port());

        let rt = tokio::runtime::Runtime::new()?;
        let app_password =
            rt.block_on(retrive_app_password(&address, "test_user", "real_password"))?;
        assert_eq!(
            "M1DqHwuZWwjEC3ku7gJsspR7bZXopwf01kj0XGppYVzEkGtbZBRaXlOUxFZdbgJ6Zk9OwG9x",
            app_password
        );
        let logout_result = rt.block_on(logout(&address, "test_user", &app_password));
        assert!(logout_result);
        Ok(())
    }

    fn mock_login_nextcloud_server() -> Result<TestServer> {
        let server = TestServer::new()?;
        let port = server.port();
        let endpoint = format!("http://127.0.0.1:{}/login/v2/poll", port);

        server
            .create_resource("/index.php/login/v2")
            .status(Status::OK)
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body_fn(move |_| {
                format!(
                    r#"{{"poll": {{ "token": "asdf", "endpoint": "{}"}}, "login": "login"}}"#,
                    endpoint
                )
            });

        server.create_resource("/login/v2/poll")
            .status(Status::OK)
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(r#"{"server": "http://127.0.0.1", "loginName": "test_user", "appPassword":"test_app_password"}"#);

        Ok(server)
    }

    fn mock_app_password_nextcloud_server() -> Result<TestServer> {
        let server = TestServer::new()?;
        server.create_resource("/ocs/v2.php/core/getapppassword")
            .status(Status::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(r#"<?xml version="1.0"?>
<ocs>
        <meta>
                <status>ok</status>
                <statuscode>200</statuscode>
                <message>OK</message>
        </meta>
        <data>
                <apppassword>M1DqHwuZWwjEC3ku7gJsspR7bZXopwf01kj0XGppYVzEkGtbZBRaXlOUxFZdbgJ6Zk9OwG9x</apppassword>
        </data>
</ocs>"#);

        server
            .create_resource("/ocs/v2.php/core/apppassword")
            .status(Status::OK)
            .method(Method::DELETE)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(
                r#"<?xml version="1.0"?>
<ocs>
        <meta>
                <status>ok</status>
                <statuscode>200</statuscode>
                <message>OK</message>
        </meta>
        <data/>
</ocs>"#,
            );

        Ok(server)
    }
}
