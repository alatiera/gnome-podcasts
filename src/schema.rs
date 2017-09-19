table! {
    episode (id) {
        id -> Integer,
        title -> Text,
        uri -> Text,
        local_uri -> Nullable<Text>,
        description -> Nullable<Text>,
        published_date -> Text,
        epoch -> Integer,
        length -> Nullable<Integer>,
        guid -> Nullable<Text>,
        podcast_id -> Integer,
    }
}

table! {
    podcast (id) {
        id -> Integer,
        title -> Text,
        uri -> Text,
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
