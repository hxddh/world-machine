from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()

old = '''fn validate_mind_profile(profile: String) -> Result<String, std::io::Error> {\n    if is_valid_mind_profile(&profile) {\n        Ok(profile)\n    } else {\n        Err(std::io::Error::other(\n            "mind profile must be a non-empty <=64 character label using a-z, 0-9, '.', '_' or '-'",\n        ))\n    }\n}\n\nfn is_valid_mind_profile(profile: &str) -> bool {\n    !profile.is_empty()\n        && profile.len() <= 64\n        && profile.bytes().all(|byte| {\n            byte.is_ascii_lowercase()\n                || byte.is_ascii_digit()\n                || matches!(byte, b'.' | b'_' | b'-')\n        })\n}\n'''
new = '''fn validate_mind_profile(profile: String) -> Result<String, std::io::Error> {\n    if is_valid_mind_profile(&profile) {\n        Ok(profile)\n    } else {\n        Err(std::io::Error::other(\n            "mind profile must be one of: deterministic, pi, custom",\n        ))\n    }\n}\n\nfn is_valid_mind_profile(profile: &str) -> bool {\n    matches!(\n        profile,\n        DETERMINISTIC_MIND_PROFILE | "pi" | CUSTOM_MIND_PROFILE\n    )\n}\n'''
if old not in s:
    raise SystemExit('slug profile validation block not found')
s = s.replace(old, new, 1)

# Compare two supported provenance profiles while forcing the same Care action.
s = s.replace('            "mind-a",\n', '            DETERMINISTIC_MIND_PROFILE,\n', 1)
s = s.replace('            "mind-b",\n', '            "pi",\n', 1)
s = s.replace('            Some(&Value::Text("mind-a".into()))\n', '            Some(&Value::Text(DETERMINISTIC_MIND_PROFILE.into()))\n', 1)
s = s.replace('        assert_eq!(profile.left.as_deref(), Some("mind-a"));\n        assert_eq!(profile.right.as_deref(), Some("mind-b"));\n', '        assert_eq!(profile.left.as_deref(), Some(DETERMINISTIC_MIND_PROFILE));\n        assert_eq!(profile.right.as_deref(), Some("pi"));\n', 1)

old_test = '''    #[test]\n    fn mind_profile_rejects_secret_shaped_or_freeform_values() {\n        let error = PocketUniverse::with_agent_runtime_profile(PocketMind, "pi api-key=secret")\n            .err()\n            .expect("freeform mind profile must be rejected");\n        assert!(error.to_string().contains("mind profile must be"));\n    }\n'''
new_test = '''    #[test]\n    fn mind_profile_rejects_arbitrary_slug_and_credential_shaped_values() {\n        for profile in [\n            "mind-a",\n            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",\n            "0123456789abcdef0123456789abcdef",\n            "pi api-key=secret",\n        ] {\n            let error = PocketUniverse::with_agent_runtime_profile(PocketMind, profile)\n                .err()\n                .expect("non-closed-set mind profile must be rejected");\n            assert!(error.to_string().contains("mind profile must be one of"));\n        }\n    }\n'''
if old_test not in s:
    raise SystemExit('mind profile direct rejection test not found')
s = s.replace(old_test, new_test, 1)

old_reg_test = '''    #[test]\n    fn registration_profile_rejects_secret_shaped_or_freeform_values_without_panicking() {\n        let error = pocket_universe_registration_with_agent_runtime_profile(\n            || PocketMind,\n            "pi api-key=secret",\n        )\n        .err()\n        .expect("freeform registration mind profile must be rejected");\n        assert!(error.to_string().contains("mind profile must be"));\n    }\n'''
new_reg_test = '''    #[test]\n    fn registration_profile_rejects_credentials_without_panicking() {\n        for profile in [\n            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",\n            "0123456789abcdef0123456789abcdef",\n        ] {\n            let error = pocket_universe_registration_with_agent_runtime_profile(\n                || PocketMind,\n                profile,\n            )\n            .err()\n            .expect("credential-shaped registration profile must be rejected");\n            assert!(error.to_string().contains("mind profile must be one of"));\n        }\n    }\n'''
if old_reg_test not in s:
    raise SystemExit('registration profile rejection test not found')
s = s.replace(old_reg_test, new_reg_test, 1)

p.write_text(s)
