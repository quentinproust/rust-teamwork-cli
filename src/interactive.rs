use crate::teamwork_config::{TeamWorkConfig, star_task, get_config, unstar_task, is_starred_task};
use crate::teamwork_service::{TeamWorkService, Project, TaskList, Task};
use dialoguer::{Select, Input, Confirmation};
use chrono::NaiveDate;

pub struct InteractiveService<'a> {
    service: TeamWorkService<'a>,
}

impl<'a> InteractiveService<'a> {
    pub fn new(config: &TeamWorkConfig) -> InteractiveService {
        let service = TeamWorkService::new(&config);
        return InteractiveService {
            service: service.clone(),
        };
    }

    pub fn handle(&self) {
        let commands = &[
            InteractiveCommand::SeeStarredTasks,
            InteractiveCommand::SearchTask,
        ];

        let selected_action = Select::new()
            .with_prompt("What do you want to do ?")
            .items(commands)
            .default(0)
            .interact()
            .expect("Failed to get action");

        match &commands[selected_action] {
            InteractiveCommand::SeeStarredTasks => self.handle_see_starred_tasks(),
            InteractiveCommand::SearchTask => self.handle_search_task(),
        }
    }

    fn handle_see_starred_tasks(&self) {
        let config = get_config()
            .expect("Could not get config")
            .expect("No config yet");

        let starred_tasks: Vec<Task> = config.starred_tasks.iter()
            .map(|task_id| self.service.get_task(&task_id)
                .expect(format!("Could not get task #{}", task_id).as_str())
            )
            .collect();

        let select_task = Select::new()
            .with_prompt("Choose a task ?")
            .items(starred_tasks.as_slice())
            .default(0)
            .interact()
            .expect("Failed to get action");

        let task = starred_tasks.get(select_task)
            .expect("Could not get selected selected task");

        self.handle_selected_task(&task);
    }

    fn handle_search_task(&self) {
        let seach_opt: Option<String> = None;
        let projects = self.service.list_project(&seach_opt)
            .expect("Could not list projects")
            .projects;

        let selected_project = Select::new()
            .with_prompt("Choose a project ?")
            .paged(true)
            .items(projects.as_slice())
            .default(0)
            .interact()
            .expect("Failed to get selected project");

        let project = projects.get(selected_project)
            .expect("Could not get selected project");

        self.handle_selected_project(&project);
    }

    fn handle_selected_project(&self, project: &Project) {
        let tasklists_list = self.service.list_tasklists(project)
            .expect(format!("Could not list tasklists of project {}", project.name).as_str());

        let select_tasklist = Select::new()
            .with_prompt("Choose a task list ?")
            .paged(true)
            .items(tasklists_list.as_slice())
            .default(0)
            .interact()
            .expect("Failed to get action");

        let tasklist = tasklists_list.get(select_tasklist)
            .expect("Could not get selected selected tasklist");

        self.handle_selected_tasklist(&tasklist)
    }

    fn handle_selected_tasklist(&self, tasklist: &TaskList) {
        let task_list_response = self.service.list_task(tasklist);
        let task_list = match task_list_response {
            Ok(r) => r,
            Err(err) => panic!("Could not list tasks of tasklist : {}", err)
        };
        //.expect(format!("Could not list tasks of tasklist {}", tasklist.name).as_str());

        let tasks = flatten_tasks(task_list);

        let select_task = Select::new()
            .with_prompt("Choose a task ?")
            .paged(true)
            .items(tasks.as_slice())
            .default(0)
            .interact()
            .expect("Failed to get action");

        let task = tasks.get(select_task)
            .expect("Could not get selected selected task");

        self.handle_selected_task(&task.task);
    }

    fn handle_selected_task(&self, task: &Task) {
        let star_command = match is_starred_task(&task.id) {
            Ok(is_starred) => match is_starred {
                true => Commands::UnstarTask(&task),
                false => Commands::StarTask(&task),
            },
            Err(err) => panic!("Could not know if task {} is starred : {}", task.id, err)
        };

        let actions = &[
            Commands::EnterTimeEntry(&task),
            star_command,
        ];

        let select_task = Select::new()
            .with_prompt("What do you want to do ?")
            .items(actions)
            .default(0)
            .interact()
            .expect("Failed to get action");

        match actions[select_task] {
            Commands::Back => println!("Not implemented yet !"),
            Commands::StarTask(t) => {
                match star_task(t.id) {
                    Ok(()) => println!("Task was starred !"),
                    Err(err) => println!("Could not star task {}", err),
                }
            }
            Commands::UnstarTask(t) => {
                match unstar_task(&t.id) {
                    Ok(()) => println!("Task was unstarred !"),
                    Err(err) => println!("Could not unstar task {}", err),
                }
            }
            Commands::EnterTimeEntry(t) => self.handle_new_time_entry(&t)
        }
    }

    fn handle_new_time_entry(&self, task: &Task) {
        let config = get_config().unwrap().unwrap();

        let default_date = self.service.last_time_entries(1, None)
            .map(|tes| tes.first()
                .map(|te| te.date.date().naive_local()))
            .unwrap_or_else(|_err| None)
            .map(|date| date.succ())
            .map(|date| date.format("%Y-%m-%d").to_string());

        let mut start_date_input = Input::<String>::new();
        start_date_input.with_prompt("Start date ?");
        if let Some(date) = default_date {
            start_date_input.default(date);
        }
        let start_date_str = start_date_input.interact()
            .unwrap();
        let start_date = NaiveDate::parse_from_str(start_date_str.as_str(), "%Y-%m-%d")
            .expect("Could not parse date");

        let hours_str = Input::<String>::new().with_prompt("Hours ?")
            .interact()
            .unwrap();
        let hours = hours_str.parse::<i32>().unwrap();

        let description = Input::<String>::new().with_prompt("Description ?")
            .interact()
            .unwrap();

        let dry_run = Confirmation::new().with_text("Dry run ?")
            .interact()
            .unwrap();

        let mut confirm = true;
        if !dry_run {
            confirm = Confirmation::new().with_text("Are you sure ?")
                .interact()
                .unwrap();
        }

        if confirm {
            self.service.save_time(
                task.id.to_string(),
                start_date,
                hours,
                description,
                dry_run,
                &config.times_off.iter(),
            ).expect("Could not save time");
        }
    }
}

fn flatten_tasks(task_list: Vec<Task>) -> Vec<TaskItem> {
    let mut tasks = vec![];

    for t in task_list {
        tasks.push(TaskItem { task: t.clone(), is_sub: false });
        for st in t.sub_tasks.clone() {
            tasks.push(TaskItem { task: st.clone(), is_sub: true });
        }
    }

    return tasks.clone();
}

enum Commands<'a> {
    // TODO Dealing with back command, it needs to deal with call stack
    Back,
    StarTask(&'a Task),
    UnstarTask(&'a Task),
    EnterTimeEntry(&'a Task),
}

impl<'a> ToString for Commands<'a> {
    fn to_string(&self) -> String {
        return match self {
            Commands::Back => "Go Back".to_string(),
            Commands::StarTask(_t) => "Star the task".to_string(),
            Commands::UnstarTask(_t) => "Unstar the task".to_string(),
            Commands::EnterTimeEntry(_t) => "Enter a time entry".to_string(),
        };
    }
}

#[derive(Debug, Clone)]
struct TaskItem {
    task: Task,
    is_sub: bool,
}

impl ToString for TaskItem {
    fn to_string(&self) -> String {
        let t = &self.task;
        return match self.is_sub {
            true => format!("\t {}", t.name),
            false => format!("{} ({} sub tasks)", t.name, t.sub_tasks.len())
        };
    }
}

impl ToString for Task {
    fn to_string(&self) -> String {
        return match self.parent_task.clone() {
            Some(p) => format!("{} > {} > {} > {}", self.project_name, self.todo_list_name, p.name, self.name),
            None => format!("{} > {} > {}", self.project_name, self.todo_list_name, self.name),
        };
    }
}

impl ToString for Project {
    fn to_string(&self) -> String {
        return self.name.clone();
    }
}

impl ToString for TaskList {
    fn to_string(&self) -> String {
        return format!("{} ({} tasks)", self.name, self.uncompleted_count);
    }
}

enum InteractiveCommand {
    SeeStarredTasks,
    SearchTask,
}

impl ToString for InteractiveCommand {
    fn to_string(&self) -> String {
        let str = match self {
            InteractiveCommand::SeeStarredTasks => "See starred tasks",
            InteractiveCommand::SearchTask => "Search tasks",
        };

        return str.to_string();
    }
}
