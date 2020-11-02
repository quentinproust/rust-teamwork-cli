#[macro_use]
extern crate prettytable;
extern crate reqwest;

use std::error::Error;

use chrono::{Datelike, NaiveDate, Utc};
use structopt::StructOpt;

use teamwork_config::{get_config, save_token_and_company};

use crate::console_printers::{print_projects, print_tasks, print_time_entries, print_times_off};
use crate::interactive::InteractiveService;
use crate::teamwork_config::{save_alias, save_config, TeamWorkConfig, TimeOff};
use crate::teamwork_service::TeamWorkService;

mod interactive;
mod teamwork_config;
mod teamwork_service;
mod console_printers;

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
enum Cli {
    Auth {
        #[structopt(short = "c")]
        company_id: String,
        #[structopt(short = "t")]
        token: String,
    },
    Project(ProjectCommand),
    TimeEntries(TimeEntriesCommand),
    TimeOff(TimeOffCommand),
    Interactive,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
enum ProjectCommand {
    List {
        #[structopt(short = "t")]
        token: Option<String>
    },
    Alias {
        #[structopt(short = "i")]
        id: String,
        #[structopt(short = "n")]
        name: String,
    },
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
enum TimeEntriesCommand {
    Last {
        #[structopt(short = "n", default_value = "10")]
        nb: i32
    },
    LastTasks,
    Missing {
        #[structopt(short = "s")]
        since: String,

        #[structopt(short = "i")]
        included: bool,
    },
    Save {
        #[structopt(short = "t")]
        task_id: String,
        #[structopt(short = "s")]
        start_date: String,
        #[structopt(short = "h")]
        hours: String,
        #[structopt(short = "d")]
        description: String,
        #[structopt(short = "r")]
        dry_run: bool,
    },
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
enum TimeOffCommand {
    Save {
        #[structopt(short = "d")]
        date: String,
        #[structopt(short = "h", default_value = "8")]
        hours: i32,
    },
    List {
        #[structopt(short = "y")]
        year: Option<String>,
        #[structopt(short = "m")]
        month: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::from_args();

    //println!("{:?}", args);

    match args {
        Cli::Auth { company_id, token } => {
            save_token_and_company(&company_id, &token);
            println!("Company and token saved in ~/.teamwork")
        }
        _ => {
            match get_config() {
                Ok(config) => match config {
                    Some(c) => handle_command_with_config(&c),
                    None => println!("No config file ~/.teamwork found. Init it by authenticating with command `auth`"),
                }
                Err(e) => println!("Oups ! {}", e),
            }
        }
    }

    Ok(())
}

fn handle_command_with_config(config: &TeamWorkConfig) {
    let args = Cli::from_args();
    match args {
        Cli::Project(project_cmd) => handle_project_command(project_cmd, &config),
        Cli::TimeEntries(time_entries_command) => handle_time_entries_command(time_entries_command, &config),
        Cli::TimeOff(time_off_command) => handle_time_off_command(time_off_command, &config),
        Cli::Interactive => {
            let interactive = InteractiveService::new(config);
            interactive.handle();
        }
        _ => {}
    }
}

fn handle_time_off_command(time_off_command: TimeOffCommand, config: &TeamWorkConfig) {
    match time_off_command {
        TimeOffCommand::Save { date, hours } => {
            let new_config = config.with_time_off(date, hours);
            save_config(&new_config);
        }
        TimeOffCommand::List { year: year_opt, month: month_opt } => {
            let time_off_iter = config.times_off.iter();

            let current_year = Utc::now().naive_local().year().to_string();

            let year = year_opt.unwrap_or(current_year);

            let selection_pattern = match month_opt {
                Some(month) => format!("{}-{}", year, month),
                None => year
            };

            let times_off = time_off_iter
                .filter(|t| t.date.starts_with(&selection_pattern))
                .collect::<Vec<&TimeOff>>();

            print_times_off(times_off);
        }
    }
}

fn handle_project_command(project_cmd: ProjectCommand, config: &TeamWorkConfig) {
    let service = TeamWorkService::new(config);

    match project_cmd {
        ProjectCommand::List { token } => {
            println!("List projects ...");

            match service.list_project(&token) {
                Ok(pl) => print_projects(&pl, &config),
                Err(e) => println!("Could not list project \n{:#?}", e)
            }
        }
        ProjectCommand::Alias { id, name } => {
            if let Err(e) = save_alias(&id, &name) {
                println!("Could not save alias : {}", e);
            }
        }
    }
}

fn handle_time_entries_command(time_entries_command: TimeEntriesCommand, config: &TeamWorkConfig) {
    let service = TeamWorkService::new(config);

    match time_entries_command {
        TimeEntriesCommand::Last { nb } => {
            println!("Last time entries ...");

            match service.last_time_entries(nb, None) {
                Ok(pl) => print_time_entries(&pl, &config),
                Err(e) => println!("Could not get last time entries \n{:#?}", e)
            }
        }
        TimeEntriesCommand::LastTasks => {
            println!("Last tasks ...");

            match service.last_used_tasks() {
                Ok(pl) => print_tasks(pl),
                Err(e) => println!("Could not get last used tasks \n{:#?}", e)
            }
        }
        TimeEntriesCommand::Missing { since, included: _included } => {
            println!("Getting missing entries since {} ...", since);

            let since_date = NaiveDate::parse_from_str(&since, "%Y-%m-%d")
                .expect(&format!("Could not parse {} using format %Y-%m-%d", &since));

            match service.get_missing_entries(since_date, &config.times_off.iter()) {
                Ok(missing_time) => {
                    let days = missing_time / 8;
                    let hours = missing_time % 8;

                    println!("Missing {} days and {} hours", days, hours);
                }
                Err(e) => println!("Could not get last time entries \n{:#?}", e)
            }
        }
        TimeEntriesCommand::Save { task_id, start_date, hours: time, description, dry_run } => {
            let date = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
                .expect(&format!("Could not parse {} using format %Y-%m-%d", &start_date));

            let hours = parse_time_duration(time.as_str())
                .expect(&format!("Could not parse {}. Expected format xxdyyh, for example 8d4h for 8 days and 4 hours.", &time));

            service.save_time(task_id, date, hours, description, dry_run, &config.times_off.iter())
                .expect("Fail to save times");
        }
    }
}

fn parse_time_duration(time: &str) -> Option<i32> {
    let time_regex = regex::Regex::new(r#"(([0-9]+)d)?(([0-9]+)h)?"#).unwrap();
    let hours = time_regex.captures(time)
        .and_then(|c| {
            let x = c.iter().collect::<Vec<_>>();
            if let &[_, _, None, _, None] = &*x {
                None
            } else if let &[_, _, input_days, _, input_hours] = &*x {
                let d = input_days.map(|m| m.as_str().parse::<i32>().unwrap()).unwrap_or(0);
                let h = input_hours.map(|m| m.as_str().parse::<i32>().unwrap()).unwrap_or(0);

                Some(d * 8 + h)
            } else {
                None
            }
        });
    return hours;
}
