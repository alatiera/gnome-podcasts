// parser.rs
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

use rss::extension::itunes::ITunesItemExtension;

/// Parses an Item Itunes extension and returns it's duration value in seconds.
// FIXME: Rafactor
#[allow(non_snake_case)]
pub(crate) fn parse_itunes_duration(item: Option<&ITunesItemExtension>) -> Option<i32> {
    let duration = item.map(|s| s.duration())??;

    // FOR SOME FUCKING REASON, IN THE APPLE EXTENSION SPEC
    // THE DURATION CAN BE EITHER AN INT OF SECONDS OR
    // A STRING OF THE FOLLOWING FORMATS:
    // HH:MM:SS, H:MM:SS, MM:SS, M:SS
    // LIKE WHO THE FUCK THOUGH THAT WOULD BE A GOOD IDEA.
    if let Ok(NO_FUCKING_LOGIC) = duration.parse::<i32>() {
        return Some(NO_FUCKING_LOGIC);
    };

    let mut seconds = 0;
    let fk_apple = duration.split(':').collect::<Vec<_>>();
    if fk_apple.len() == 3 {
        seconds += fk_apple[0].parse::<i32>().unwrap_or(0) * 3600;
        seconds += fk_apple[1].parse::<i32>().unwrap_or(0) * 60;
        seconds += fk_apple[2].parse::<i32>().unwrap_or(0);
    } else if fk_apple.len() == 2 {
        seconds += fk_apple[0].parse::<i32>().unwrap_or(0) * 60;
        seconds += fk_apple[1].parse::<i32>().unwrap_or(0);
    }

    Some(seconds)
}

#[cfg(test)]
mod tests {
    use rss::extension::itunes::ITunesItemExtensionBuilder;

    use super::*;

    #[test]
    fn test_itunes_duration() {
        // Input is a String<Int>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("3370".into()))
            .build()
            .unwrap();
        let item = Some(&extension);
        assert_eq!(parse_itunes_duration(item), Some(3370));

        // Input is a String<M:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("6:10".into()))
            .build()
            .unwrap();
        let item = Some(&extension);
        assert_eq!(parse_itunes_duration(item), Some(370));

        // Input is a String<MM:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("56:10".into()))
            .build()
            .unwrap();
        let item = Some(&extension);
        assert_eq!(parse_itunes_duration(item), Some(3370));

        // Input is a String<H:MM:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("1:56:10".into()))
            .build()
            .unwrap();
        let item = Some(&extension);
        assert_eq!(parse_itunes_duration(item), Some(6970));

        // Input is a String<HH:MM:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("01:56:10".into()))
            .build()
            .unwrap();
        let item = Some(&extension);
        assert_eq!(parse_itunes_duration(item), Some(6970));
    }
}
