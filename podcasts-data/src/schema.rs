#![allow(warnings)]

table! {
    episodes (title, show_id) {
        rowid -> Integer,
        title -> Text,
        uri -> Nullable<Text>,
        local_uri -> Nullable<Text>,
        description -> Nullable<Text>,
        epoch -> Integer,
        length -> Nullable<Integer>,
        duration -> Nullable<Integer>,
        play_position -> Integer,
        guid -> Nullable<Text>,
        played -> Nullable<Integer>,
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

allow_tables_to_appear_in_same_query!(episodes, shows, source);
