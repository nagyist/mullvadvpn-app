use jnix::{
    FromJava, JnixEnv,
    jni::{
        JNIEnv,
        objects::{JObject, JString},
        sys::{JNI_FALSE, JNI_TRUE, jboolean},
    },
};
use mullvad_api::ApiEndpoint;
use std::path::Path;
use talpid_types::ErrorExt;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_dataproxy_MullvadProblemReport_collectReport(
    env: JNIEnv<'_>,
    _: JObject<'_>,
    logDirectory: JString<'_>,
    outputPath: JString<'_>,
) -> jboolean {
    let env = JnixEnv::from(env);
    let log_dir_string = String::from_java(&env, logDirectory);
    let log_dir = Path::new(&log_dir_string);
    let output_path_string = String::from_java(&env, outputPath);
    let output_path = Path::new(&output_path_string);

    match mullvad_problem_report::collect_report::<&str>(&[], output_path, Vec::new(), log_dir) {
        Ok(()) => JNI_TRUE,
        Err(error) => {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to collect problem report")
            );
            JNI_FALSE
        }
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_mullvad_mullvadvpn_dataproxy_MullvadProblemReport_sendProblemReport(
    env: JNIEnv<'_>,
    _: JObject<'_>,
    userEmail: JString<'_>,
    userMessage: JString<'_>,
    outputPath: JString<'_>,
    cacheDirectory: JString<'_>,
    endpoint: JObject<'_>,
) -> jboolean {
    let env = JnixEnv::from(env);
    let user_email = String::from_java(&env, userEmail);
    let user_message = String::from_java(&env, userMessage);
    let output_path_string = String::from_java(&env, outputPath);
    let output_path = Path::new(&output_path_string);
    let cache_directory_string = String::from_java(&env, cacheDirectory);
    let cache_directory = Path::new(&cache_directory_string);
    let api_endpoint =
        crate::api::api_endpoint_from_java(&env, endpoint).unwrap_or(ApiEndpoint::from_env_vars());

    let send_result = mullvad_problem_report::send_problem_report(
        &user_email,
        &user_message,
        output_path,
        cache_directory,
        api_endpoint,
    );

    match send_result {
        Ok(()) => JNI_TRUE,
        Err(error) => {
            log::error!(
                "{}",
                error.display_chain_with_msg("Failed to send problem report")
            );
            JNI_FALSE
        }
    }
}
