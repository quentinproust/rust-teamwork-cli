use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::hash::Hash;
use std::io::Result as IoResult;
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

#[derive(Deserialize, Clone, Serialize, Debug)]
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

impl PartialEq<TeamWorkConfig> for TeamWorkConfig {
    fn eq(&self, other: &TeamWorkConfig) -> bool {
        *self.company_id == other.company_id
            && *self.token == other.token
            && array_eq(&*self.times_off, &other.times_off)
            && array_eq(&*self.starred_tasks, &other.starred_tasks)
            && array_eq(&*self.project_aliases, &other.project_aliases)
    }
}

fn array_eq<T>(a: &[T], b: &[T]) -> bool where T: Eq + Hash,
{
    if a.len() != b.len() {
        return false;
    }

    let a_hash: HashSet<_> = a.iter().collect();
    let b_hash: HashSet<_> = b.iter().collect();

    a_hash == b_hash
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectAlias {
    pub project_id: String,
    pub alias: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeOff {
    pub date: String,
    pub hours: i32,
}

pub fn get_config() -> Result<Option<TeamWorkConfig>, Box<dyn Error>> {
    let path = get_teamwork_file();
    return get_config_from_path(&path);
}

pub fn get_config_from_path(file_path: &PathBuf) -> Result<Option<TeamWorkConfig>, Box<dyn Error>> {
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

pub fn save_alias(project_id: &String, alias: &String) -> Result<TeamWorkConfig, Box<dyn Error>> {
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

pub fn is_starred_task(task_id: &usize) -> Result<bool, Box<dyn Error>> {
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

pub fn star_task(task_id: usize) -> Result<(), Box<dyn Error>> {
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

pub fn unstar_task(task_id: &usize) -> Result<(), Box<dyn Error>> {
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
    save_config_to_path(config, &get_teamwork_file())
        .expect("Unable to write file ~/.teamwork");
}

fn save_config_to_path(config: &TeamWorkConfig, path: &PathBuf) -> IoResult<()> {
    let serializable_config = SerializableTeamWorkConfig::from(config);

    let toml = serde_json::to_string_pretty(&serializable_config)
        .expect("Could not create config");

    return fs::write(path, toml);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_save_config() {
        let mut output_path = std::env::temp_dir();
        output_path.push(".teamwork-cli-config_test_can_save_config-c6b69f99-5a24-49d1-8b7d-d76f88a5c245.json");

        let config = TeamWorkConfig {
            company_id: "test-company-id".to_string(),
            token: "test-token".to_string(),
            project_aliases: vec![
                ProjectAlias {
                    alias: "project-alias-1".to_string(),
                    project_id: "project-id-1".to_string(),
                },
                ProjectAlias {
                    alias: "project-alias-1".to_string(),
                    project_id: "project-id-1".to_string(),
                }
            ],
            starred_tasks: vec![124343, 24543543],
            times_off: vec![
                TimeOff {
                    date: "2020-01-23".to_string(),
                    hours: 8,
                },
                TimeOff {
                    date: "2020-01-24".to_string(),
                    hours: 4,
                }
            ],
        };

        let result = save_config_to_path(&config, &output_path);

        assert!(!result.is_err(), "{} should have been writen without error, but got {:#?}", output_path.to_str().unwrap(), result.err());

        let result_content = fs::read_to_string(output_path);
        let expected_content = "{
  \"company_id\": \"test-company-id\",
  \"token\": \"test-token\",
  \"project_aliases\": [
    {
      \"project_id\": \"project-id-1\",
      \"alias\": \"project-alias-1\"
    },
    {
      \"project_id\": \"project-id-1\",
      \"alias\": \"project-alias-1\"
    }
  ],
  \"times_off\": [
    {
      \"date\": \"2020-01-23\",
      \"hours\": 8
    },
    {
      \"date\": \"2020-01-24\",
      \"hours\": 4
    }
  ],
  \"starred_tasks\": [
    124343,
    24543543
  ]
}";

        assert_eq!(result_content.unwrap(), expected_content);
    }

    #[test]
    fn test_can_read_config() {
        let mut output_path = std::env::temp_dir();
        output_path.push(".teamwork-cli-config_test_can_read_config-c6b69f99-5a24-49d1-8b7d-d76f88a5c245.json");

        let test_config_as_string = "{
  \"company_id\": \"test-company-id\",
  \"token\": \"test-token\",
  \"project_aliases\": [
    {
      \"project_id\": \"project-id-1\",
      \"alias\": \"project-alias-1\"
    },
    {
      \"project_id\": \"project-id-1\",
      \"alias\": \"project-alias-1\"
    }
  ],
  \"times_off\": [
    {
      \"date\": \"2020-01-23\",
      \"hours\": 8
    },
    {
      \"date\": \"2020-01-24\",
      \"hours\": 4
    }
  ],
  \"starred_tasks\": [
    124343,
    24543543
  ]
}";

        fs::write(output_path.clone(), test_config_as_string).unwrap();

        let result = get_config_from_path(&output_path);

        assert!(!result.is_err(), "should have read config from {}, but got {:#?}", output_path.to_str().unwrap(), result.err());

        let success = result.unwrap();
        assert!(success.is_some(), "should have existing config");

        let config = TeamWorkConfig {
            company_id: "test-company-id".to_string(),
            token: "test-token".to_string(),
            project_aliases: vec![
                ProjectAlias {
                    alias: "project-alias-1".to_string(),
                    project_id: "project-id-1".to_string(),
                },
                ProjectAlias {
                    alias: "project-alias-1".to_string(),
                    project_id: "project-id-1".to_string(),
                }
            ],
            starred_tasks: vec![124343, 24543543],
            times_off: vec![
                TimeOff {
                    date: "2020-01-23".to_string(),
                    hours: 8,
                },
                TimeOff {
                    date: "2020-01-24".to_string(),
                    hours: 4,
                }
            ],
        };

        assert_eq!(success.unwrap(), config);
    }
}
