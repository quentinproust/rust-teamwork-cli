use std::borrow::Borrow;

use chrono::{Datelike, DateTime, NaiveDate, Utc, Weekday};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde::de::DeserializeOwned;

use crate::teamwork_config::{TeamWorkConfig, TimeOff};
use std::slice::Iter;

const WORKING_DAY_DURATION: i32 = 8;

#[derive(Clone)]
pub struct TeamWorkService<'a> {
    client: HttpClient<'a>,
}

impl<'a> TeamWorkService<'a> {
    pub fn new<'b>(config: &'b TeamWorkConfig) -> TeamWorkService<'b> {
        let client = HttpClient::new(config);

        return TeamWorkService { client };
    }

    pub fn get_account(&self) -> Result<Account, reqwest::Error> {
        let response: AccountResponse = self.client.get("me.json")?;

        return Ok(response.account);
    }

    pub fn list_project(&self, search_opt: &Option<String>) -> Result<ProjectsResponse, reqwest::Error> {
        let projects: ProjectsResponse = match search_opt {
            Some(search_term) => self.client.get_with_params("projects.json", &[("searchTerm", search_term)])?,
            None => self.client.get("projects.json")?,
        };

        return Ok(projects);
    }

    pub fn list_tasklists(&self, project: &Project) -> Result<Vec<TaskList>, reqwest::Error> {
        let url = format!("projects/{}/tasklists.json", project.id);
        let response: TasklistsResponse = self.client
            .get(url.as_str())?;

        return Ok(response.tasklists);
    }

    pub fn list_task(&self, tasklist: &TaskList) -> Result<Vec<Task>, reqwest::Error> {
        let url = format!("tasklists/{}/tasks.json", tasklist.id);
        let response: TasksResponse = self.client
            .get_with_params(url.as_str(), &[("nestSubTasks", "yes")])?;

        return Ok(response.tasks);
    }

    pub fn last_time_entries(
        &self,
        nb_result: i32,
        start_date: Option<NaiveDate>,
    ) -> Result<Vec<TimeEntry>, reqwest::Error> {
        let account = self.get_account()?;

        println!("nb {}", nb_result);

        let from_date_opt = start_date.map(|d| d.format("%Y%m%d").to_string());

        let mut query_params = vec![
            ("userId", account.id),
            ("pageSize", nb_result.to_string()),
            ("sortby", "date".to_string()),
            ("sortorder", "DESC".to_string()),
        ];
        if let Some(date) = from_date_opt {
            query_params.push(("fromdate", date.to_string()))
        }

        let response: TimeEntriesResponse = self.client.get_with_params(
            "time_entries.json",
            query_params.as_slice(),
        )?;

        println!("nb time entries {}", response.time_entries.len());

        return Ok(response.time_entries);
    }

    pub fn last_used_tasks(&self) -> Result<Vec<Task>, reqwest::Error> {
        let time_entries = self.last_time_entries(60, None)?;

        let tasks = time_entries.iter()
            .map(|t| t.task())
            .fold(vec![], |acc, task|
                match acc.contains(&task) {
                    true => acc,
                    false => {
                        let mut vec = acc.to_vec();
                        vec.push(task);
                        return vec;
                    }
                },
            );

        return Ok(tasks);
    }


    pub fn get_missing_entries(&self, since_date: NaiveDate, times_off: &Iter<TimeOff>) -> Result<i32, reqwest::Error> {
        let today = Utc::today().naive_utc();

        if today.le(&since_date) {
            return Ok(0);
        }

        let time_entries = self.last_time_entries(500, Some(since_date))?;
        let existing_time_entries = time_entries.iter();

        let mut missing = 0;
        let mut d = since_date.clone();
        while d.lt(&today) {
            let remaining_workload = get_remaining_workload(d, &existing_time_entries, times_off);

            if is_working_day(d) {
                missing += remaining_workload;
            }
            d = d.succ();
        }

        return Ok(missing);
    }

    pub fn save_time(
        &self,
        task_id: String,
        start_date: NaiveDate,
        hours: i32,
        description: String,
        dry_run: bool,
        times_off: &Iter<TimeOff>,
    ) -> Result<i32, reqwest::Error> {
        let account = self.get_account()?;
        let account_id = account.id.as_str();

        let time_entries = self.last_time_entries(500, Some(start_date))?;
        let existing_time_entries = time_entries.iter();

        let mut current_date = start_date.clone();
        let today = &Utc::today().naive_utc();

        let mut remaining_input_hours = hours.clone();

        println!("Start adding time entries. Remaining hours : {}", remaining_input_hours);

        while current_date.lt(today) && remaining_input_hours > 0 {
            let remaining_workload = get_remaining_workload(current_date, &existing_time_entries, times_off);

            println!("{} / {} : {}",
                     current_date.format("%Y%m%d"),
                     remaining_workload,
                     description);
            if !dry_run {
                let new_time_entry = TimeEntryInput {
                    date: current_date.format("%Y%m%d").to_string(),
                    time: "08:00".to_string(),
                    hours: remaining_workload.to_string(),
                    description: description.clone(),
                    minutes: "0".to_string(),
                    person_id: account_id.to_string(),
                };

                let response = self.save_time_entry(task_id.clone(), &new_time_entry)?;
                let id = response.id.unwrap_or_else(|| "unknown".to_string());
                match response.status.as_str() {
                    "OK" => println!("\t ✔️ (#id : {})", id),
                    _ => {
                        println!("\t ❓ {} (#id : {})", response.status, id);
                    }
                }
            }

            remaining_input_hours -= remaining_workload;

            current_date = current_date.succ();
            while !is_working_day(current_date) {
                current_date = current_date.succ();
            }
        }

        return Ok(hours);
    }

    pub fn save_time_entry(&self, task_id: String, time_entry: &TimeEntryInput) -> Result<TimeEntryCreatedResponse, reqwest::Error> {
        let value = serde_json::to_value(time_entry)
            .expect("Could not parse time entry to json value");

        let body = json!({
            "time-entry": value
        });

        let path = format!("/tasks/{}/time_entries.json", task_id);
        return self.client.post(path.as_str(), &body);
    }

// projects http 'http://altima1.eu.teamwork.com/projects.json' authorization:'basic dHdwXzlrM3NoOXFQU1RPUU03QnJISWRDMUFzSlo3WXRfZXU6eHh4'
// project's tasks http 'http://altima1.eu.teamwork.com/projects/359738/tasks.json' authorization:'basic dHdwXzlrM3NoOXFQU1RPUU03QnJISWRDMUFzSlo3WXRfZXU6eHh4'
// create a time entry for a task https://developer.teamwork.com/projects/time-tracking/create-a-time-entry-for-a-task
}

fn is_working_day(d: NaiveDate) -> bool {
    return d.weekday() != Weekday::Sat && d.weekday() != Weekday::Sun;
}

fn get_remaining_workload(
    date: NaiveDate,
    existing_time_entries: &Iter<TimeEntry>,
    times_off: &Iter<TimeOff>,
) -> i32 {
    let existings = existing_time_entries
        .clone()
        .filter(|t| t.date.date().naive_utc() == date);

    let mut remaining_workload = WORKING_DAY_DURATION;
    for e in existings {
        remaining_workload -= e.hours();
    }

    let tos = times_off
        .clone()
        .filter(|t| t.date == date.format("%Y-%m-%d").to_string());

    for t in tos {
        remaining_workload -= t.hours;
    }

    if remaining_workload < 0 {
        remaining_workload = 0;
    }

    return remaining_workload;
}

#[derive(Debug, Deserialize)]
pub struct ProjectsResponse {
    #[serde(alias = "STATUS")]
    pub status: String,
    pub projects: Vec<Project>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AccountResponse {
    #[serde(alias = "STATUS")]
    pub status: String,
    #[serde(alias = "person")]
    pub account: Account,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct TasklistsResponse {
    #[serde(alias = "STATUS")]
    pub status: String,
    pub tasklists: Vec<TaskList>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TaskList {
    pub id: String,
    pub name: String,
    //"pinned": true,
    #[serde(alias = "uncompleted-count")]
    pub uncompleted_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct TimeEntriesResponse {
    #[serde(alias = "STATUS")]
    pub status: String,
    #[serde(alias = "time-entries")]
    pub time_entries: Vec<TimeEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TimeEntry {
    pub id: String,
    pub description: String,
    pub date: DateTime<Utc>,
    pub hours: String,
    #[serde(alias = "project-id")]
    pub project_id: String,
    #[serde(alias = "project-name")]
    pub project_name: String,
    #[serde(alias = "todo-list-id")]
    pub todo_list_id: String,
    #[serde(alias = "todo-list-name")]
    pub todo_list_name: String,
    #[serde(alias = "todo-item-id")]
    pub todo_item_id: String,
    #[serde(alias = "todo-item-name")]
    pub todo_item_name: String,
}

impl TimeEntry {
    pub fn hours(&self) -> i32 {
        return self.hours.parse::<i32>().unwrap();
    }

    pub fn task(&self) -> Task {
        let id = self.todo_item_id.clone();
        let name = self.todo_item_name.clone();
        return Task { id: id.parse().unwrap(), name, sub_tasks: vec![] };
    }
}

#[derive(Debug, Serialize)]
pub struct TimeEntryInput {
    pub description: String,
    #[serde(rename = "person-id")]
    pub person_id: String,
    pub date: String,
    pub time: String,
    pub hours: String,
    pub minutes: String,
}

#[derive(Debug, Deserialize)]
pub struct TimeEntryCreatedResponse {
    #[serde(alias = "timeLogId")]
    pub id: Option<String>,
    #[serde(alias = "STATUS")]
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct TasksResponse {
    #[serde(alias = "STATUS")]
    pub status: String,
    #[serde(alias = "todo-items")]
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Task {
    pub id: usize,
    #[serde(alias = "content")]
    pub name: String,
    #[serde(default, alias = "subTasks")]
    pub sub_tasks: Vec<Task>,
}

#[derive(Clone)]
struct HttpClient<'a> {
    company_id: &'a str,
    token: &'a str,
}

impl<'a> HttpClient<'a> {
    fn new<'b>(config: &'b TeamWorkConfig) -> HttpClient<'b> {
        return HttpClient {
            company_id: &config.company_id,
            token: &config.token,
        };
    }

    fn post<O, T: ?Sized>(&self, path: &str, body: &T) -> Result<O, reqwest::Error>
        where O: DeserializeOwned,
              T: Serialize
    {
        let url = format!("https://{}.eu.teamwork.com/{}", self.company_id, path);

        let body_as_string = serde_json::to_string(body)
            .expect("Could not serialize to json");

        let client = reqwest::Client::new();
        let no_password: Option<String> = None;
        let body: O = client.post(url.as_str())
            .basic_auth(self.token, no_password)
            .body(body_as_string)
            .send()?
            .json()?;

        return Ok(body);
    }

    fn get<O>(&self, path: &str) -> Result<O, reqwest::Error> where O: DeserializeOwned {
        let url = format!("https://{}.eu.teamwork.com/{}", self.company_id, path);

        let client = reqwest::Client::new();
        let no_password: Option<String> = None;
        let body: O = client.get(url.as_str())
            .basic_auth(self.token, no_password)
            .send()?
            .json()?;

        return Ok(body);
    }

    fn get_with_params<I, K, V, O>(&self, path: &str, query_params: I) -> Result<O, reqwest::Error>
        where I: IntoIterator,
              I::Item: Borrow<(K, V)>,
              K: AsRef<str>,
              V: AsRef<str>,
              O: DeserializeOwned {
        let url = format!("https://{}.eu.teamwork.com/{}", self.company_id, path);

        let with_params = Url::parse_with_params(&url, query_params)
            .expect("Could not parse url");

        println!("url {}", with_params);

        let client = reqwest::Client::new();
        let no_password: Option<String> = None;
        let body: O = client.get(with_params.as_str())
            .basic_auth(self.token, no_password)
            .send()?
            .json()?;

        return Ok(body);
    }
}
