const RPM_LIKE: [&str; 9] =
    ["fedora", "rhel", "centos", "rocky", "almalinux", "suse", "opensuse", "sles", "mageia"];
const DEB_LIKE: [&str; 8] =
    ["debian", "ubuntu", "linuxmint", "pop", "elementary", "kali", "raspbian", "zorin"];

fn matches_family<'a>(mut tokens: impl Iterator<Item = &'a str>, family: &[&str]) -> bool {
    tokens.any(|token| family.contains(&token))
}

#[tauri::command]
pub fn detect_linux_package_type() -> String {
    #[cfg(target_os = "linux")]
    {
        use os_release::OsRelease;

        let Ok(os_release) = OsRelease::new() else {
            return "unknown".to_string();
        };

        let id = os_release.id.trim().to_lowercase();
        let id_like = os_release.id_like.trim().to_lowercase();
        let tokens = id.split_whitespace().chain(id_like.split_whitespace());

        if matches_family(tokens.clone(), &RPM_LIKE) {
            return "rpm".to_string();
        }

        if matches_family(tokens, &DEB_LIKE) {
            return "deb".to_string();
        }

        "unknown".to_string()
    }

    #[cfg(not(target_os = "linux"))]
    {
        "unknown".to_string()
    }
}
