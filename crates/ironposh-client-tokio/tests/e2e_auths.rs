pub fn auths_from_env_or_default() -> Vec<&'static str> {
    // Keep the default set small and reliable for local runs.
    //
    // Users can override locally:
    // `IRONPOSH_E2E_AUTHS=basic,ntlm,negotiate,kerberos`
    let env = std::env::var("IRONPOSH_E2E_AUTHS").ok();
    if let Some(env) = env {
        let v = env
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if v.is_empty() {
            return vec!["basic"];
        }

        // Leak to static to keep the signature simple for tests. This is fine for test processes.
        return v
            .into_iter()
            .map(|s| Box::leak(s.into_boxed_str()) as &str)
            .collect();
    }

    vec!["basic"]
}
