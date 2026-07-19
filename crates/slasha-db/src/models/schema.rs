// @generated automatically by Diesel CLI.

diesel::table! {
    alert_channels (id) {
        id -> Text,
        name -> Text,
        kind -> Text,
        config_json -> Text,
        enabled -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    alert_incidents (id) {
        id -> Text,
        rule_id -> Text,
        target_key -> Text,
        status -> Text,
        trigger_value -> Nullable<Double>,
        current_value -> Nullable<Double>,
        recovery_value -> Nullable<Double>,
        threshold_value -> Nullable<Double>,
        opened_at -> Timestamp,
        last_notified_at -> Nullable<Timestamp>,
        resolved_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    alert_notifications (id) {
        id -> Text,
        incident_id -> Text,
        kind -> Text,
        message -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    alert_rules (id) {
        id -> Text,
        name -> Text,
        config_json -> Text,
        channel_ids_json -> Text,
        direct_webhook_url -> Nullable<Text>,
        message_template -> Nullable<Text>,
        shell_command -> Nullable<Text>,
        enabled -> Bool,
        cooldown_secs -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

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
        created_at -> Timestamp,
        updated_at -> Timestamp,
        last_checked_at -> Nullable<Timestamp>,
        last_check_ok -> Nullable<Bool>,
        last_check_error -> Nullable<Text>,
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
        created_at -> Timestamp,
        auto_deploy -> Bool,
        source -> Text,
        node_id -> Text,
    }
}

diesel::table! {
    cron_jobs (id) {
        id -> Text,
        app_id -> Text,
        name -> Text,
        schedule -> Text,
        command -> Text,
        timezone -> Text,
        enabled -> Bool,
        timeout_secs -> Integer,
        last_run_at -> Nullable<Timestamp>,
        next_run_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        runtime -> Text,
    }
}

diesel::table! {
    cron_runs (id) {
        id -> Text,
        cron_job_id -> Text,
        status -> Text,
        trigger_kind -> Text,
        exit_code -> Nullable<Integer>,
        error -> Nullable<Text>,
        started_at -> Nullable<Timestamp>,
        finished_at -> Nullable<Timestamp>,
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
        node_id -> Text,
    }
}

diesel::table! {
    git_connections (app_id) {
        app_id -> Text,
        clone_url -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    github_app_config (id) {
        id -> Text,
        app_id -> Text,
        client_id -> Text,
        client_secret -> Text,
        private_key -> Text,
        webhook_secret -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    github_connections (app_id) {
        app_id -> Text,
        installation_id -> BigInt,
        repository_id -> BigInt,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    github_installations (user_id, installation_id) {
        user_id -> Text,
        installation_id -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    nodes (id) {
        id -> Text,
        name -> Text,
        host -> Nullable<Text>,
        user -> Nullable<Text>,
        port -> Nullable<Integer>,
        ssh_private_key -> Nullable<Text>,
        internal_root_ca -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamp>,
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

diesel::joinable!(alert_incidents -> alert_rules (rule_id));
diesel::joinable!(alert_notifications -> alert_incidents (incident_id));
diesel::joinable!(app_backups -> apps (app_id));
diesel::joinable!(app_domains -> apps (app_id));
diesel::joinable!(app_env_vars -> apps (app_id));
diesel::joinable!(app_members -> apps (app_id));
diesel::joinable!(app_members -> users (user_id));
diesel::joinable!(app_scale -> apps (app_id));
diesel::joinable!(apps -> nodes (node_id));
diesel::joinable!(cron_jobs -> apps (app_id));
diesel::joinable!(cron_runs -> cron_jobs (cron_job_id));
diesel::joinable!(deployments -> apps (app_id));
diesel::joinable!(deployments -> nodes (node_id));
diesel::joinable!(git_connections -> apps (app_id));
diesel::joinable!(github_connections -> apps (app_id));
diesel::joinable!(github_installations -> users (user_id));
diesel::joinable!(service_env_vars -> services (service_id));
diesel::joinable!(services -> apps (app_id));
diesel::joinable!(ssh_keys -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    alert_channels,
    alert_incidents,
    alert_notifications,
    alert_rules,
    app_backups,
    app_domains,
    app_env_vars,
    app_members,
    app_scale,
    apps,
    cron_jobs,
    cron_runs,
    deployments,
    git_connections,
    github_app_config,
    github_connections,
    github_installations,
    nodes,
    service_env_vars,
    services,
    ssh_keys,
    users,
);
