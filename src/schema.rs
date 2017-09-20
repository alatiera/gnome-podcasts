table! {
    episode (id) {
        id -> Integer,
        title -> Nullable<Text>,
        uri -> Nullable<Text>,
        local_uri -> Nullable<Text>,
        description -> Nullable<Text>,
        published_date -> Nullable<Text>,
        epoch -> Nullable<Integer>,
        length -> Nullable<Integer>,
        guid -> Nullable<Text>,
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
