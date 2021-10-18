use regex::Regex;
use rusoto_credential::{AwsCredentials, CredentialsError};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

pub mod helper;

pub fn parse_credentials_file(
    file_path: &Path,
) -> Result<HashMap<String, AwsCredentials>, CredentialsError> {
    match fs::metadata(file_path) {
        Err(_) => {
            return Err(CredentialsError::new(format!(
                "Couldn't stat credentials file: [ {:?} ]. Non existant, or no permission.",
                file_path
            )))
        }
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(CredentialsError::new(format!(
                    "Credentials file: [ {:?} ] is not a file.",
                    file_path
                )));
            }
        }
    };

    let file = File::open(file_path)?;

    let profile_regex = new_profile_regex();
    let mut profiles: HashMap<String, AwsCredentials> = HashMap::new();
    let mut access_key: Option<String> = None;
    let mut secret_key: Option<String> = None;
    let mut token: Option<String> = None;
    let mut profile_name: Option<String> = None;

    let file_lines = BufReader::new(&file);
    for (line_no, line) in file_lines.lines().enumerate() {
        let unwrapped_line: String =
            line.unwrap_or_else(|_| panic!("Failed to read credentials file, line: {}", line_no));

        // skip empty lines
        if unwrapped_line.is_empty() {
            continue;
        }

        // skip comments
        if unwrapped_line.starts_with('#') {
            continue;
        }

        // handle the opening of named profile blocks
        if profile_regex.is_match(&unwrapped_line) {
            if let (Some(profile_name_value), Some(access_key_value), Some(secret_key_value)) =
                (profile_name, access_key, secret_key)
            {
                let creds = AwsCredentials::new(access_key_value, secret_key_value, token, None);

                profiles.insert(profile_name_value, creds);
            }

            access_key = None;
            secret_key = None;
            token = None;

            let caps = profile_regex.captures(&unwrapped_line).unwrap();
            profile_name = Some(caps.get(2).unwrap().as_str().to_string());
            continue;
        }

        // otherwise look for key=value pairs we care about
        let lower_case_line = unwrapped_line.to_ascii_lowercase().to_string();

        if lower_case_line.contains("aws_access_key_id") && access_key.is_none() {
            let v: Vec<&str> = unwrapped_line.split('=').collect();
            if !v.is_empty() {
                access_key = Some(v[1].trim_matches(' ').to_string());
            }
        } else if lower_case_line.contains("aws_secret_access_key") && secret_key.is_none() {
            let v: Vec<&str> = unwrapped_line.split('=').collect();
            if !v.is_empty() {
                secret_key = Some(v[1].trim_matches(' ').to_string());
            }
        } else if lower_case_line.contains("aws_session_token") && token.is_none() {
            let v: Vec<&str> = unwrapped_line.split('=').collect();
            if !v.is_empty() {
                token = Some(v[1].trim_matches(' ').to_string());
            }
        } else if lower_case_line.contains("aws_security_token") {
            if token.is_none() {
                let v: Vec<&str> = unwrapped_line.split('=').collect();
                if !v.is_empty() {
                    token = Some(v[1].trim_matches(' ').to_string());
                }
            }
        } else {
            // Ignore unrecognized fields
            continue;
        }
    }

    if let (Some(profile_name_value), Some(access_key_value), Some(secret_key_value)) =
        (profile_name, access_key, secret_key)
    {
        let creds = AwsCredentials::new(access_key_value, secret_key_value, token, None);

        profiles.insert(profile_name_value, creds);
    }

    if profiles.is_empty() {
        return Err(CredentialsError::new("No credentials found."));
    }

    Ok(profiles)
}

fn new_profile_regex() -> Regex {
    Regex::new(r"^\[(profile )?([^\]]+)\]$").expect("Failed to compile regex")
}

#[cfg(test)]
mod tests {

    use crate::helper::file::config::create_profile_config_map_from;
    use std::path::Path;

    const DEFAULT: &str = "default";
    const REGION: &str = "region";

    #[test]
    fn parse_config_file_credential_process() {
        let result = create_profile_config_map_from(Path::new(
            "tests/sample-data/credential_process_config",
        ));
        assert!(result.is_some());
        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 2);
        let default_profile = profiles
            .get(DEFAULT)
            .expect("No Default profile in default_profile_credentials");
        assert_eq!(default_profile.get(REGION), Some(&"us-east-1".to_string()));
        assert_eq!(
            default_profile.get("credential_process"),
            Some(&"cat tests/sample-data/credential_process_sample_response".to_string())
        );
    }

    #[test]
    fn parse_credentials_file_default_profile() {
        let result = super::parse_credentials_file(Path::new(
            "tests/sample-data/default_profile_credentials",
        ));
        assert!(result.is_ok());

        let profiles = result.ok().unwrap();
        assert_eq!(profiles.len(), 1);

        let default_profile = profiles
            .get(DEFAULT)
            .expect("No Default profile in default_profile_credentials");
        assert_eq!(default_profile.aws_access_key_id(), "foo");
        assert_eq!(default_profile.aws_secret_access_key(), "bar");
    }

    #[test]
    fn parse_credentials_file_multiple_profiles() {
        let result = super::parse_credentials_file(Path::new(
            "tests/sample-data/multiple_profile_credentials",
        ));
        assert!(result.is_ok());

        let profiles = result.ok().unwrap();
        assert_eq!(profiles.len(), 2);

        let foo_profile = profiles
            .get("foo")
            .expect("No foo profile in multiple_profile_credentials");
        assert_eq!(foo_profile.aws_access_key_id(), "foo_access_key");
        assert_eq!(foo_profile.aws_secret_access_key(), "foo_secret_key");

        let bar_profile = profiles
            .get("bar")
            .expect("No bar profile in multiple_profile_credentials");
        assert_eq!(bar_profile.aws_access_key_id(), "bar_access_key");
        assert_eq!(bar_profile.aws_secret_access_key(), "bar_secret_key");
    }

    #[test]
    fn parse_all_values_credentials_file() {
        let result =
            super::parse_credentials_file(Path::new("tests/sample-data/full_profile_credentials"));
        assert!(result.is_ok());

        let profiles = result.ok().unwrap();
        assert_eq!(profiles.len(), 1);

        let default_profile = profiles
            .get(DEFAULT)
            .expect("No default profile in full_profile_credentials");
        assert_eq!(default_profile.aws_access_key_id(), "foo");
        assert_eq!(default_profile.aws_secret_access_key(), "bar");
    }
}