pub mod aws_profile_credential;

use std::collections::HashMap;
use std::fs::{self};
use std::io::BufRead;
use std::path::Path;

use rusoto_credential::{AwsCredentials, CredentialsError};

use crate::file::create_file_reader_for;
use crate::file::credential::aws_profile_credential::AwsProfileCredential;
use crate::file::helper::line::{extract_value_from, is_comment_or_empty};
use crate::file::helper::line::{get_profile_name_from, is_profile};

pub fn parse_credentials_file(
    credential_file_path: &Path,
) -> Result<HashMap<String, AwsCredentials>, CredentialsError> {
    match is_valid_file_path(credential_file_path) {
        Ok(_) => {
            let profile_credentials_map = create_profile_credentials_map_from(credential_file_path);

            if profile_credentials_map.is_empty() {
                return Err(CredentialsError::new("No credentials found."));
            }

            Ok(profile_credentials_map)
        }
        Err(e) => Err(e),
    }
}

fn is_valid_file_path(credential_file_path: &Path) -> Result<(), CredentialsError> {
    match fs::metadata(credential_file_path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(CredentialsError::new(format!(
                    "Credentials file: [ {:?} ] is not a file.",
                    credential_file_path
                )));
            }
        }
        Err(_) => {
            return Err(CredentialsError::new(format!(
                "Couldn't stat credentials file: [ {:?} ]. Non existent, or no permission.",
                credential_file_path
            )))
        }
    };

    Ok(())
}

fn create_profile_credentials_map_from(
    credential_file_path: &Path,
) -> HashMap<String, AwsCredentials> {
    let credential_file_reader = create_file_reader_for(credential_file_path);

    let mut profile_credentials_map: HashMap<String, AwsCredentials> = HashMap::new();
    let mut aws_profile_credential = AwsProfileCredential::new();

    for (line_no, line) in credential_file_reader.lines().enumerate() {
        let unwrapped_line: String =
            line.unwrap_or_else(|_| panic!("Failed to read credentials file, line: {}", line_no));

        if is_comment_or_empty(&unwrapped_line) {
            continue;
        }

        if is_profile(&unwrapped_line) {
            profile_credentials_map =
                try_insert_profile_credential_to(profile_credentials_map, aws_profile_credential);

            aws_profile_credential = AwsProfileCredential::new_with_profile_name(
                get_profile_name_from(&unwrapped_line)
                    .unwrap_or_else(|| panic!("Cannot get profile name, line: {}", line_no)),
            );
        } else {
            aws_profile_credential =
                try_assign_aws_profile_credential_from(&unwrapped_line, aws_profile_credential);
        }
    }

    profile_credentials_map =
        try_insert_profile_credential_to(profile_credentials_map, aws_profile_credential);

    profile_credentials_map
}

fn try_assign_aws_profile_credential_from(
    line: &str,
    mut aws_profile_credential: AwsProfileCredential,
) -> AwsProfileCredential {
    let lower_case_line = line.to_ascii_lowercase();

    if is_aws_access_key(&lower_case_line) && aws_profile_credential.access_key.is_none() {
        aws_profile_credential.access_key = extract_value_from(&lower_case_line);
    } else if is_aws_secret_key(&lower_case_line) && aws_profile_credential.secret_key.is_none() {
        aws_profile_credential.secret_key = extract_value_from(&lower_case_line);
    } else if is_aws_token(&lower_case_line) && aws_profile_credential.token.is_none() {
        aws_profile_credential.token = extract_value_from(&lower_case_line);
    }

    aws_profile_credential
}

fn is_aws_access_key(line: &str) -> bool {
    line.contains("aws_access_key_id")
}

fn is_aws_secret_key(line: &str) -> bool {
    line.contains("aws_secret_access_key")
}

fn is_aws_token(line: &str) -> bool {
    line.contains("aws_session_token") || line.contains("aws_security_token")
}

fn try_insert_profile_credential_to(
    mut profile_credentials_map: HashMap<String, AwsCredentials>,
    aws_profile_credential: AwsProfileCredential,
) -> HashMap<String, AwsCredentials> {
    if let (Some(profile_name), Some(aws_credential)) = (
        aws_profile_credential.profile_name.clone(),
        aws_profile_credential.into_aws_credential(),
    ) {
        profile_credentials_map.insert(profile_name, aws_credential);
    }

    profile_credentials_map
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::file::config::create_profile_config_map_from;

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
