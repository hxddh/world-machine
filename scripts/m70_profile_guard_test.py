from pathlib import Path

path = Path("worlds/micro-company/src/lib.rs")
text = path.read_text()
anchor = '''    #[test]
    fn forking_before_company_resolution_reopens_the_company() {
'''
test = r'''    #[test]
    fn mind_profile_rejects_arbitrary_and_credential_shaped_values() {
        for profile in [
            "provider-model-x",
            "ghp_1234567890abcdef1234567890abcdef",
            "0123456789abcdef0123456789abcdef",
        ] {
            let error = MicroCompany::with_agent_runtime_profile(
                MockAgentRuntime::scripted(Vec::<String>::new()),
                profile,
            )
            .err()
            .expect("unsafe mind profile must be rejected");
            assert!(error
                .to_string()
                .contains("deterministic, pi, custom"));

            let registration = micro_company_registration_with_agent_runtime_profile(
                || MockAgentRuntime::scripted(Vec::<String>::new()),
                profile,
            );
            assert!(registration.is_err());
        }
    }

'''
if text.count(anchor) != 1:
    raise SystemExit(f"expected one test anchor, found {text.count(anchor)}")
path.write_text(text.replace(anchor, test + anchor, 1))
