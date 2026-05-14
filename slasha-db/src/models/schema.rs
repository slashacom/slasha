// @generated automatically by Diesel CLI.

diesel::table! {
    app_domains (id) {
        id -> Text,
        app_id -> Text,
        domain -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    app_env_vars (id) {
        id -> Text,
        app_id -> Text,
        key -> Text,
        value -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    app_members (app_id, user_id) {
        app_id -> Text,
        user_id -> Text,
        role -> Text,
        added_at -> Timestamp,
    }
}

diesel::table! {
    app_scale (id) {
        id -> Text,
        app_id -> Text,
        process_type -> Text,
        desired -> Integer,
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
    deployments (id) {
        id -> Text,
        app_id -> Text,
        commit_sha -> Text,
        commit_message -> Text,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    service_env_vars (id) {
        id -> Text,
        service_id -> Text,
        key -> Text,
        value -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    services (id) {
        id -> Text,
        app_id -> Text,
        kind -> Text,
        name -> Text,
        version -> Text,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
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

diesel::joinable!(app_domains -> apps (app_id));
diesel::joinable!(app_env_vars -> apps (app_id));
diesel::joinable!(app_members -> apps (app_id));
diesel::joinable!(app_members -> users (user_id));
diesel::joinable!(app_scale -> apps (app_id));
diesel::joinable!(deployments -> apps (app_id));
diesel::joinable!(service_env_vars -> services (service_id));
diesel::joinable!(services -> apps (app_id));
diesel::joinable!(ssh_keys -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    app_domains,
    app_env_vars,
    app_members,
    app_scale,
    apps,
    deployments,
    service_env_vars,
    services,
    ssh_keys,
    users,
);
