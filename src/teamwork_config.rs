extern crate toml;

use serde::{Serialize, Deserialize};
use std::fs;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::marker::PhantomData;
//use serde::export::PhantomData;

#[derive(Debug, Clone)]
pub struct NoConfigError;

impl fmt::Display for NoConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "no config file ~/.teamwork")
    }
}

impl Error for NoConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Deserialize, Clone, Serialize)]
pub struct TeamWorkConfig<'a> {
    pub company_id: String,
    pub token: String,
    pub project_aliases: Vec<ProjectAlias>,
    pub times_off: Vec<TimeOff>,
    _m: Option<PhantomData<&'a str>>,
}

impl<'a> TeamWorkConfig<'a> {
    pub fn get_alias(&self, project_id: &String) -> Option<&ProjectAlias> {
        return self.project_aliases.iter()
            .find(|a| a.project_id.as_str() == project_id.as_str());
    }

    pub fn with_time_off(&self, date: String, hours: i32) -> TeamWorkConfig {
        let off = TimeOff {
            date: date.clone(),
            hours: hours.clone(),
        };
        let mut new = self.clone();
        new.times_off.retain(|time_off| time_off.date != date);

        if hours > 0 {
            new.times_off.push(off);
        }

        return new;
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ProjectAlias {
    pub project_id: String,
    pub alias: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TimeOff {
    pub date: String,
    pub hours: i32,
}

pub fn get_config<'a>() -> Result<Option<TeamWorkConfig<'a>>, Box<Error>> {
    let file_path = get_teamwork_file();

    if !file_path.exists() {
        return Ok(None);
    }

    let file_content = fs::read_to_string(file_path)?;

    let config: TeamWorkConfig = toml::from_str(&file_content)?;

    return Ok(Some(config));
}

pub fn save_token_and_company(company_id: &String, token: &String) {
    let config = TeamWorkConfig {
        company_id: company_id.clone(),
        token: token.clone(),
        project_aliases: vec![],
        times_off: vec![],
        _m: None
    };
    save_config(&config);
}

pub fn save_alias<'a>(project_id: &String, alias: &String) -> Result<TeamWorkConfig<'a>, Box<Error>> {
    match get_config() {
        Ok(config) => match config {
            Some(c) => {
                let new_alias = ProjectAlias {
                    project_id: project_id.clone(),
                    alias: alias.clone(),
                };
                let mut aliases = c.project_aliases.to_vec();
                aliases.push(new_alias);

                let tc = TeamWorkConfig {
                    project_aliases: aliases,
                    ..c
                };

                save_config(&tc);
                Ok(tc)
            }
            None => Err(Box::new(NoConfigError)),
        }
        Err(e) => Err(e),
    }
}

pub fn save_config(config: &TeamWorkConfig) {
    let toml = toml::to_string(config)
        .expect("Could not create config");

    fs::write(get_teamwork_file(), toml)
        .expect("Unable to write file ~/.teamwork");
}

fn get_teamwork_file() -> PathBuf {
    let home_dir = dirs::home_dir()
        .expect("Could not get your home dir");

    return home_dir.join(".teamwork");
}

/*
read write file : https://stackoverflow.com/questions/31192956/whats-the-de-facto-way-of-reading-and-writing-files-in-rust-1-x
*/
