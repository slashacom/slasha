// @generated automatically by Diesel CLI.

diesel::table! {
    app_members (app_id, user_id) {
        app_id -> Text,
        user_id -> Text,
        role -> Text,
        added_at -> Timestamp,
    }
}

diesel::table! {
    apps (id) {
        id -> Text,
        slug -> Text,
        name -> Text,
        repo_path -> Text,
        default_branch -> Text,
        status -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    ssh_keys (id) {
        id -> Text,
        user_id -> Text,
        title -> Nullable<Text>,
        public_key -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Text,
        email -> Text,
        password_hash -> Text,
        role -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(app_members -> apps (app_id));
diesel::joinable!(app_members -> users (user_id));
diesel::joinable!(ssh_keys -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(app_members, apps, ssh_keys, users,);
