#![allow(warnings)]

table! {
    episodes (id) {
        id -> Integer,
        title -> Text,
        uri -> Nullable<Text>,
        local_uri -> Nullable<Text>,
        description -> Nullable<Text>,
        image_uri -> Nullable<Text>,
        epoch -> Timestamp,
        length -> Nullable<Integer>,
        duration -> Nullable<Integer>,
        guid -> Nullable<Text>,
        played -> Nullable<Timestamp>,
        play_position -> Integer,
        show_id -> Integer,
    }
}

table! {
    shows (id) {
        id -> Integer,
        title -> Text,
        link -> Text,
        description -> Text,
        image_uri -> Nullable<Text>,
        image_uri_hash -> Nullable<Binary>,
        image_cached -> Timestamp,
        source_id -> Integer,
    }
}

table! {
    source (id) {
        id -> Integer,
        uri -> Text,
        last_modified -> Nullable<Text>,
        http_etag -> Nullable<Text>,
    }
}

table! {
    discovery_settings (platform_id) {
        platform_id -> Text,
        enabled -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(episodes, shows, source, discovery_settings);
