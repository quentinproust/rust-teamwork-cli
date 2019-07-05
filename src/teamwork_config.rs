use std::error::Error;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
pub struct TeamWorkConfig {
    pub company_id: String,
    pub token: String,
    pub project_aliases: Vec<ProjectAlias>,
    pub times_off: Vec<TimeOff>,
    pub starred_tasks: Vec<usize>,
}

impl TeamWorkConfig {
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
        let mut times_off = new.times_off;
        times_off.retain(|time_off| time_off.date != date);

        if hours > 0 {
            times_off.push(off);
        }

        new.times_off = times_off;

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

pub fn get_config() -> Result<Option<TeamWorkConfig>, Box<Error>> {
    let file_path = get_teamwork_file();

    if !file_path.exists() {
        return Ok(None);
    }

    let file_content = fs::read_to_string(file_path)?;

    let serializable_config: SerializableTeamWorkConfig = serde_json::from_str(&file_content)?;
    let config = TeamWorkConfig::from(serializable_config);

    return Ok(Some(config));
}

pub fn save_token_and_company(company_id: &String, token: &String) {
    let config = TeamWorkConfig {
        company_id: company_id.clone(),
        token: token.clone(),
        project_aliases: vec![],
        times_off: vec![],
        starred_tasks: vec![],
    };
    save_config(&config);
}

pub fn save_alias(project_id: &String, alias: &String) -> Result<TeamWorkConfig, Box<Error>> {
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

pub fn is_starred_task(task_id: &usize) -> Result<bool, Box<Error>> {
    match get_config() {
        Ok(config) => match config {
            Some(c) => {
                let is_found = c.starred_tasks.iter().find(|t| t == &task_id)
                    .map(|_| true)
                    .unwrap_or_else(|| false);

                Ok(is_found)
            }
            None => Err(Box::new(NoConfigError)),
        }
        Err(e) => Err(e),
    }
}

pub fn star_task(task_id: usize) -> Result<(), Box<Error>> {
    match get_config() {
        Ok(config) => match config {
            Some(c) => {
                let mut tasks = c.starred_tasks.to_vec();
                tasks.push(task_id);

                let tc = TeamWorkConfig {
                    starred_tasks: tasks,
                    ..c
                };

                save_config(&tc);
                Ok(())
            }
            None => Err(Box::new(NoConfigError)),
        }
        Err(e) => Err(e),
    }
}

pub fn unstar_task(task_id: &usize) -> Result<(), Box<Error>> {
    match get_config() {
        Ok(config) => match config {
            Some(c) => {
                let mut tasks = c.starred_tasks.to_vec();
                tasks.retain(|t| t != task_id);

                let tc = TeamWorkConfig {
                    starred_tasks: tasks,
                    ..c
                };

                save_config(&tc);
                Ok(())
            }
            None => Err(Box::new(NoConfigError)),
        }
        Err(e) => Err(e),
    }
}

pub fn save_config(config: &TeamWorkConfig) {
    let serializable_config = SerializableTeamWorkConfig::from(config);

    let toml = serde_json::to_string(&serializable_config)
        .expect("Could not create config");

    fs::write(get_teamwork_file(), toml)
        .expect("Unable to write file ~/.teamwork");
}

fn get_teamwork_file() -> PathBuf {
    let home_dir = dirs::home_dir()
        .expect("Could not get your home dir");

    return home_dir.join(".teamwork");
}

#[derive(Deserialize, Clone, Serialize)]
pub struct SerializableTeamWorkConfig {
    pub company_id: String,
    pub token: String,
    project_aliases: Option<Vec<ProjectAlias>>,
    times_off: Option<Vec<TimeOff>>,
    starred_tasks: Option<Vec<usize>>,
}

impl From<&TeamWorkConfig> for SerializableTeamWorkConfig {
    fn from(config: &TeamWorkConfig) -> Self {
        let c = config.clone();

        return SerializableTeamWorkConfig {
            company_id: c.company_id,
            token: c.token,
            project_aliases: Some(c.project_aliases),
            times_off: Some(c.times_off),
            starred_tasks: Some(c.starred_tasks),
        };
    }
}

impl From<SerializableTeamWorkConfig> for TeamWorkConfig {
    fn from(config: SerializableTeamWorkConfig) -> Self {
        return TeamWorkConfig {
            company_id: config.company_id,
            token: config.token,
            project_aliases: config.project_aliases.unwrap_or_else(|| vec![]),
            times_off: config.times_off.unwrap_or_else(|| vec![]),
            starred_tasks: config.starred_tasks.unwrap_or_else(|| vec![]),
        };
    }
}
