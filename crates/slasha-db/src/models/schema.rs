// @generated automatically by Diesel CLI.

diesel::table! {
    app_backups (id) {
        id -> Text,
        app_id -> Text,
        enabled -> Bool,
        db_path -> Text,
        bucket -> Text,
        endpoint -> Text,
        path_prefix -> Nullable<Text>,
        access_key_id -> Text,
        secret_access_key -> Text,
        restore_pending -> Bool,
        last_synced_at -> Nullable<Timestamp>,
        last_checked_at -> Nullable<Timestamp>,
        last_check_ok -> Nullable<Bool>,
        last_check_error -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

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
    app_metrics (id) {
        id -> Text,
        app_id -> Text,
        cpu_usage -> Float,
        memory_used -> Integer,
        memory_limit -> Integer,
        network_rx_bps -> Float,
        network_tx_bps -> Float,
        disk_read_bps -> Float,
        disk_write_bps -> Float,
        created_at -> Timestamp,
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
        resources -> Nullable<Text>,
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

diesel::joinable!(app_backups -> apps (app_id));
diesel::joinable!(app_domains -> apps (app_id));
diesel::joinable!(app_env_vars -> apps (app_id));
diesel::joinable!(app_members -> apps (app_id));
diesel::joinable!(app_members -> users (user_id));
diesel::joinable!(app_metrics -> apps (app_id));
diesel::joinable!(app_scale -> apps (app_id));
diesel::joinable!(deployments -> apps (app_id));
diesel::joinable!(service_env_vars -> services (service_id));
diesel::joinable!(services -> apps (app_id));
diesel::joinable!(ssh_keys -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    app_backups,
    app_domains,
    app_env_vars,
    app_members,
    app_metrics,
    app_scale,
    apps,
    deployments,
    service_env_vars,
    services,
    ssh_keys,
    users,
);
