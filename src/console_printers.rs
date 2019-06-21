use prettytable::Table;
use crate::teamwork_service::{ProjectsResponse, TimeEntry, Task};
use crate::teamwork_config::{TeamWorkConfig, TimeOff};

pub fn print_projects(project_response: &ProjectsResponse, config: &TeamWorkConfig) {
    let mut table = Table::new();
    table.add_row(row!["#id", "Alias", "Name"]);

    for p in project_response.projects.iter() {
        let alias = config.get_alias(&p.id)
            .map(|a| a.alias.as_str())
            .unwrap_or( "--");
        table.add_row(row![p.id, alias, p.name]);
    }

    table.print_tty(true);
}

pub fn print_time_entries(entries: &Vec<TimeEntry>, _config: &TeamWorkConfig) {
    let mut table = Table::new();
    table.add_row(row!["#id", "Date", "Task", "Description", "Hours"]);

    for e in entries.iter() {
        let date = e.date.format("%d-%m-%Y").to_string();

        let task_desc = format!("{}\n> {}\n> {}", e.project_name, e.todo_list_name, e.todo_item_name);
        table.add_row(row![e.id, date, task_desc, e.description, e.hours()]);
    }

    table.print_tty(true);
}

pub fn print_tasks(tasks: Vec<Task>) {
    let mut table = Table::new();
    table.add_row(row!["Id", "Name"]);

    for t in tasks {
        table.add_row(row![t.id, t.name]);
    }

    table.print_tty(true);
}

pub fn print_times_off(times_off: Vec<&TimeOff>) {
    let mut table = Table::new();
    table.add_row(row!["Date", "Hours"]);

    let mut ts = times_off.clone();
    ts.sort_by(|t1, t2| t2.date.cmp(&t1.date));

    for t in ts {
        table.add_row(row![t.date, t.hours]);
    }

    table.print_tty(true);
}
