use crate::cmdline::Opt;

use awsp::helper::file::config::{create_profile_config_map_from, get_aws_config_file_path};

use dialoguer::{theme::ColorfulTheme, Select};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::{collections::HashMap, process};
use sysinfo::{get_current_pid, ProcessExt, Signal, System, SystemExt};

const REGIONS_DISPLAY: &[&str] = &[
    "us-east-2      | Ohio",
    "us-east-1      | N. Virginia",
    "us-west-1      | N. California",
    "us-west-2      | Oregon",
    "ap-south-1     | Mumbai",
    "ap-northeast-3 | Osaka-Local",
    "ap-northeast-2 | Seoul",
    "ap-northeast-1 | Tokyo",
    "ap-southeast-1 | Singapore",
    "ap-southeast-2 | Sydney",
    "ca-central-1   | Central",
    "cn-north-1     | Beijing",
    "cn-nortwest-1  | Ningxia",
    "eu-central-1   | Frankfurt",
    "eu-west-1      | Ireland",
    "eu-west-2      | London",
    "eu-west-3      | Paris",
    "eu-north-1     | Stockholm",
    "sa-east-1      | São Paulo",
];

const REGIONS: &[&str] = &[
    "us-east-2",
    "us-east-1",
    "us-west-1",
    "us-west-2",
    "ap-south-1",
    "ap-northeast-3",
    "ap-northeast-2",
    "ap-northeast-1",
    "ap-southeast-1",
    "ap-southeast-2",
    "ca-central-1",
    "cn-north-1",
    "cn-nortwest-1",
    "eu-central-1",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "eu-north-1",
    "sa-east-1",
];

const AWS_DEFAULT_PROFILE: &str = "AWS_PROFILE";
const AWS_DEFAULT_REGION: &str = "AWS_DEFAULT_REGION";
const VERSION: &str = env!("CARGO_PKG_VERSION");

// TODO Error Handler
// pub fn run(opt: &Opt) -> Result<(), Box<dyn Error>> {
pub fn run(opt: &Opt) {
    if opt.version {
        print!("\nawsp: ");
        green_ln!("{}\n", VERSION);
        process::exit(1);
    } else if opt.region {
        region_menu();
    } else {
        profile_menu();
        region_menu();
    }

    display_selected();

    exec_process();

    // TODO Error Handler
    // Ok(())
}

fn display_selected() {
    // clear screen charactor
    print!("{esc}c", esc = 27 as char);
    green!("\n ->");
    print!("  Profile: ");
    green!("{}", default_env("AWS_PROFILE"));
    print!(" | Region: ");
    green_ln!("{} \n", default_env("AWS_DEFAULT_REGION"));
}

fn profile_menu() {
    let location = get_aws_config_file_path().unwrap();
    let config_file = create_profile_config_map_from(location.as_path()).unwrap();
    let profile_list = to_key_list(&config_file);
    let profile_list = profile_list.as_slice();
    let default_profile = default_env("AWS_PROFILE");
    let display_prompt = format!("profile (current: {} )", default_profile);
    let selection = display(display_prompt, profile_list, 0);
    select_profile(profile_list[selection]);
}

fn region_menu() {
    let default_region = default_env("AWS_DEFAULT_REGION");
    let display_prompt = format!("region (current: {} )", default_region);
    let selection = display(display_prompt, REGIONS_DISPLAY, 0);
    select_region(REGIONS[selection]);
}

fn exec_process() {
    Command::new(find_shell().unwrap())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    terminate_parent_process();
}

fn default_env(env: &str) -> String {
    match env::var(env) {
        Ok(env) => env,
        Err(_) => String::from(""),
    }
}

fn to_key_list<K, V>(map: &HashMap<K, V>) -> Vec<&K> {
    let mut key_list = Vec::new();

    for key in map.keys() {
        key_list.push(key);
    }

    key_list
}

fn find_shell() -> Option<PathBuf> {
    let current_pid = get_current_pid().ok().unwrap();
    let s = System::new_all();
    let current_process = s.process(current_pid)?;
    let parent_pid = current_process.parent()?;
    let parent_process = s.process(parent_pid)?;
    let shell_path = parent_process.exe();
    Some(shell_path.to_path_buf())
}

fn terminate_parent_process() {
    let pid = get_current_pid().ok().unwrap();
    let s = System::new_all();
    let current_process = s.process(pid).unwrap();
    let parent_pid = current_process.parent().unwrap();
    let parent_process = s.process(parent_pid).unwrap();
    parent_process.kill(Signal::Kill);
}

fn display<T: ToString>(display_prompt: String, list: &[T], default: usize) -> usize {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(display_prompt)
        .default(default)
        .items(list)
        .paged(true)
        .interact()
        .unwrap()
}

fn select_profile(profile: &str) {
    env::set_var(AWS_DEFAULT_PROFILE, profile);
}

fn select_region(region: &str) {
    env::set_var(AWS_DEFAULT_REGION, region);
}

// TODO manage stack process when run awsp multiple
// fn is_terminate_previous_process() -> Option<bool> {
//     let s = System::new_all();
//     let current_pid = get_current_pid().ok()?;
//     let current_process = s.process(current_pid)?;
//     let current_path = current_process.exe();
//     let parent_pid = current_process.parent()?;
//     let parent_process = s.process(parent_pid)?;
//     let parent_of_parent_pid = parent_process.parent()?;
//     let parent_of_parent_process = s.process(parent_of_parent_pid)?;
//     let parent_of_parent_path = parent_of_parent_process.exe();
//     let current_path = current_path.to_str().unwrap();
//     let parent_of_parent_path = parent_of_parent_path.to_str().unwrap();
//     dbg!(current_path);
//     dbg!(parent_of_parent_path);
//     if current_path.eq(parent_of_parent_path) {
//         parent_of_parent_process.kill(Signal::Kill);
//         return Some(true);
//     }
//     Some(false)
// }

#[cfg(test)]
mod tests {

    use super::*;
    use std::env;

    #[test]
    fn select_profile_with_selection() {
        select_profile("ped");
        let result = env::var("AWS_PROFILE").unwrap();
        let expect = String::from("ped");
        assert_eq!(expect, result);
    }

    #[test]
    fn select_region_with_selection() {
        select_region("ped");
        let result = env::var("AWS_DEFAULT_REGION").unwrap();
        let expect = String::from("ped");
        assert_eq!(expect, result);
    }

    #[test]
    fn parse_convert_to_map_test() {
        let mut map = HashMap::new();
        map.insert(String::from("key_1"), "ABC");
        map.insert(String::from("key_2"), "50");
        map.insert(String::from("key_3"), "value");
        let result = to_key_list(&map);

        assert!(result.iter().any(|&key| key == "key_1"));
        assert!(result.iter().any(|&key| key == "key_2"));
        assert!(result.iter().any(|&key| key == "key_3"));
    }

    // Flaky test
    // #[test]
    // fn parse_default_env_no_value() {
    //     let result = default_env("CHECK");
    //     let expect = String::from("");
    //     assert_eq!(expect, result);
    // }

    #[test]
    fn parse_default_env_has_value() {
        env::set_var("CHECK", "value");
        let result = default_env("CHECK");
        let expect = String::from("value");
        assert_eq!(expect, result);
    }
}