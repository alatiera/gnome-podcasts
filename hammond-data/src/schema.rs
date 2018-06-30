table! {
    episode (title, podcast_id) {
        rowid -> Integer,
        title -> Text,
        uri -> Nullable<Text>,
        local_uri -> Nullable<Text>,
        description -> Nullable<Text>,
        epoch -> Integer,
        length -> Nullable<Integer>,
        duration -> Nullable<Integer>,
        guid -> Nullable<Text>,
        played -> Nullable<Integer>,
        podcast_id -> Integer,
    }
}

table! {
    podcast (id) {
        id -> Integer,
        title -> Text,
        link -> Text,
        description -> Text,
        image_uri -> Nullable<Text>,
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

allow_tables_to_appear_in_same_query!(episode, podcast, source);
