use dirs::home_dir;
use regex::Regex;
use rusoto_credential::CredentialsError;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::{collections::HashMap, env::var};

const AWS_CONFIG_FILE_ENV_VAR_NAME: &str = "AWS_CONFIG_FILE";
const DEFAULT_AWS_CONFIG_FILE_PATH: &str = ".aws/config";

/// Default config file location:
/// 1: if set and not empty, use the value from environment variable `AWS_CONFIG_FILE`
/// 2. otherwise return `~/.aws/config` (Linux/Mac) resp. `%USERPROFILE%\.aws\config` (Windows)
pub fn get_aws_config_file_path() -> Result<PathBuf, CredentialsError> {
    let env = try_get_env_variable_value_from(AWS_CONFIG_FILE_ENV_VAR_NAME);
    match env {
        Some(path) => Ok(PathBuf::from(path)),
        None => get_default_aws_config_file_path(),
    }
}

fn try_get_env_variable_value_from(env_variable_name: &str) -> Option<String> {
    match var(env_variable_name) {
        Ok(value) => {
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        }
        Err(_) => None,
    }
}

fn get_default_aws_config_file_path() -> Result<PathBuf, CredentialsError> {
    match home_dir() {
        Some(home_path) => {
            let home_path_str = home_path
                .to_str()
                .expect("Cannot parse home directory to &str.");
            let default_aws_config_file_path =
                format!("{}/{}", home_path_str, DEFAULT_AWS_CONFIG_FILE_PATH);

            Ok(PathBuf::from(default_aws_config_file_path))
        }
        None => Err(CredentialsError::new("Failed to determine home directory.")),
    }
}

/// Create profile -> configs map from target file.
///
/// # Argument
///
/// `config_file_path` - path to aws profile config
///
/// # Return value
///
/// `None` - If destination path is not a file otherwise return [profile -> configs] hashmap
pub fn create_profile_config_map_from(
    config_file_path: &Path,
) -> Option<HashMap<String, HashMap<String, String>>> {
    if !config_file_path.is_file() {
        return None;
    }

    let config_file = File::open(config_file_path).expect("expected file");
    let config_file_reader = BufReader::new(&config_file);

    _create_profile_config_map_from(config_file_reader)
}

fn _create_profile_config_map_from(
    config_file_reader: BufReader<&File>,
) -> Option<HashMap<String, HashMap<String, String>>> {
    let result: (HashMap<String, HashMap<String, String>>, Option<String>) = config_file_reader
        .lines()
        .filter_map(|line| try_get_config_line_from(line.ok()))
        .fold(Default::default(), |(config_map, profile), line| {
            if is_profile(&line) {
                (config_map, get_profile_from(&line))
            } else {
                match extract_config_from(&line) {
                    (key, value) if !key.is_empty() && !value.is_empty() => {
                        let config_map = insert_config_to_correspond_profile(
                            key.to_string(),
                            value.to_string(),
                            profile.clone(),
                            config_map,
                        );

                        (config_map, profile)
                    }
                    _ => (config_map, profile),
                }
            }
        });

    Some(result.0)
}

fn is_profile(line: &str) -> bool {
    let profile_regex = new_profile_regex();

    profile_regex.is_match(line)
}

fn get_profile_from(line: &str) -> Option<String> {
    let profile_regex = new_profile_regex();
    let caps = profile_regex.captures(&line).unwrap();

    caps.get(2).map(|value| value.as_str().to_string())
}

fn new_profile_regex() -> Regex {
    Regex::new(r"^\[(profile )?([^\]]+)\]$").expect("Failed to compile regex")
}

fn try_get_config_line_from(maybe_config_line: Option<String>) -> Option<String> {
    maybe_config_line.filter(|line| {
        let line = line.trim().to_owned();
        !is_comment(&line) && !line.is_empty()
    })
}

fn is_comment(to_check: &str) -> bool {
    to_check.starts_with('#')
}

fn extract_config_from(line: &str) -> (&str, &str) {
    let config_map = line
        .splitn(2, '=')
        .map(|value| value.trim())
        .collect::<Vec<&str>>();

    (config_map[0], config_map[1])
}

fn insert_config_to_correspond_profile(
    key: String,
    value: String,
    profile: Option<String>,
    mut config_map: HashMap<String, HashMap<String, String>>,
) -> HashMap<String, HashMap<String, String>> {
    if let Some(current_profile_name) = profile {
        let current_profile = config_map
            .entry(current_profile_name)
            .or_insert_with(HashMap::new);
        (*current_profile).insert(key, value);
    }

    config_map
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::env::{remove_var, set_var};
    use std::path::Path;

    const DEFAULT: &str = "default";
    const REGION: &str = "region";

    #[test]
    fn create_profile_config_map_from_should_create_profile_config_map_correctly_when_given_config_with_one_profile(
    ) {
        let result =
            super::create_profile_config_map_from(Path::new("tests/sample-data/default_config"));
        assert!(result.is_some());
        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 1);
        let default_profile = profiles
            .get(DEFAULT)
            .expect("No Default profile in default_profile_credentials");
        assert_eq!(default_profile.get(REGION), Some(&"us-east-2".to_string()));
        assert_eq!(default_profile.get("output"), Some(&"json".to_string()));
    }

    #[test]
    fn create_profile_config_map_from_should_create_profile_config_map_correctly_when_given_config_with_multiple_profiles(
    ) {
        let result = super::create_profile_config_map_from(Path::new(
            "tests/sample-data/multiple_profile_config",
        ));
        assert!(result.is_some());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 3);

        let foo_profile = profiles
            .get("foo")
            .expect("No foo profile in multiple_profile_credentials");
        assert_eq!(foo_profile.get(REGION), Some(&"us-east-3".to_string()));
        assert_eq!(foo_profile.get("output"), Some(&"json".to_string()));

        let bar_profile = profiles
            .get("bar")
            .expect("No bar profile in multiple_profile_credentials");
        assert_eq!(bar_profile.get(REGION), Some(&"us-east-4".to_string()));
        assert_eq!(bar_profile.get("output"), Some(&"json".to_string()));
    }

    #[test]
    fn create_profile_config_map_from_should_not_parse_comments_to_config_map_when_given_config_with_comments(
    ) {
        let result = super::create_profile_config_map_from(Path::new(
            "tests/sample-data/multiple_profile_config",
        ));

        let profiles = result.unwrap();

        let bar_profile = profiles
            .get("bar")
            .expect("No bar profile in multiple_profile_credentials");

        assert_eq!(bar_profile.contains_key("comments"), false);
    }

    #[test]
    fn try_get_env_variable_value_from_should_return_none_when_given_not_exist_variable_name() {
        let result = try_get_env_variable_value_from(
            "some_nonsense_name_which_should_not_be_one_of_env_var_name",
        );

        assert_eq!(result, None);
    }

    #[test]
    fn try_get_env_variable_value_from_should_return_value_when_given_existing_env_variable_name() {
        let env_var_name = "some_nonsense_key_name_with_random_id_0x4567";
        let env_var_value = "someValue";

        set_var(env_var_name, env_var_value);

        let result = try_get_env_variable_value_from(env_var_name);

        assert_eq!(result.unwrap(), env_var_value);

        remove_var(env_var_name);
    }

    // This test is to make sure that default aws config path will not be changed by mistake.
    #[test]
    fn get_default_aws_config_file_path_should_return_expected_default_path_when_called() {
        let result = get_default_aws_config_file_path();

        let home_dir_path_buf = home_dir().expect("Cannot get home directory.");
        let home_dir = home_dir_path_buf
            .to_str()
            .expect("Cannot parse home directory to &str.");

        let expected = format!("{}/.aws/config", home_dir);

        assert_eq!(result.unwrap(), PathBuf::from(expected));
    }

    #[test]
    fn parse_config_file_should_return_none_when_given_path_is_not_exist() {
        let result = create_profile_config_map_from(Path::new("some/nonsense/path"));

        assert_eq!(result, None);
    }

    #[test]
    fn try_get_config_line_from_should_return_none_when_given_line_is_comment() {
        let input = Some("# comment".to_owned());

        let result = try_get_config_line_from(input);

        assert_eq!(result, None);
    }

    #[test]
    fn try_get_config_line_from_should_return_none_when_given_line_is_empty() {
        let input = Some("".to_owned());

        let result = try_get_config_line_from(input);

        assert_eq!(result, None);
    }

    #[test]
    fn try_get_config_line_from_should_return_result_when_given_proper_config_line() {
        let input = Some("someConfig".to_owned());

        let result = try_get_config_line_from(input.clone());

        assert_eq!(result, input);
    }
}