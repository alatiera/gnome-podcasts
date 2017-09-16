table! {
    episode (id) {
        id -> Integer,
        title -> Text,
        uri -> Text,
        local_uri -> Nullable<Text>,
        description -> Nullable<Text>,
        epoch -> Integer,
        length -> Integer,
        guid -> Nullable<Text>,
        podcast_id -> Integer,
    }
}

table! {
    podcast (id) {
        id -> Integer,
        title -> Text,
        uri -> Text,
        link -> Nullable<Text>,
        description -> Nullable<Text>,
        last_modified -> Nullable<Text>,
        http_etag -> Nullable<Text>,
        image_uri -> Nullable<Text>,
    }
}
