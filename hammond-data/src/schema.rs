table! {
    episode (title, podcast_id) {
        rowid -> Integer,
        title -> Text,
        uri -> Nullable<Text>,
        local_uri -> Nullable<Text>,
        description -> Nullable<Text>,
        published_date -> Nullable<Text>,
        epoch -> Integer,
        length -> Nullable<Integer>,
        guid -> Nullable<Text>,
        played -> Nullable<Integer>,
        favorite -> Bool,
        archive -> Bool,
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
        favorite -> Bool,
        archive -> Bool,
        always_dl -> Bool,
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
