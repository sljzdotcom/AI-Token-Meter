fn main() {
    tauri_build::build();

    println!("cargo:rerun-if-changed=tests/fixtures/code0-native-fixture.rc");
    println!("cargo:rerun-if-changed=tests/fixtures/code0-native-fixture.manifest");

    // This opt-in debug profile resource is for supported test targets only.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        && std::env::var_os("CARGO_FEATURE_CODE0_NATIVE_FIXTURE").is_some()
        && std::env::var("PROFILE").as_deref() == Ok("debug")
    {
        embed_resource::compile_for_tests(
            "tests/fixtures/code0-native-fixture.rc",
            embed_resource::ParamsIncludeDirs(["tests/fixtures"]),
        )
        .manifest_required()
        .expect("failed to compile Code0 native fixture manifest resource");
    }
}
