table! {
    entries (id) {
        id -> Integer,
        parent -> Integer,
        entry_type -> Text,
        key -> Text,
        label -> Text,
        url -> Nullable<Text>,
        extra -> Nullable<Text>,
        provider_extra -> Nullable<Text>,
    }
}
