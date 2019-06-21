use crate::teamwork_config::TeamWorkConfig;
use crate::teamwork_service::{TeamWorkService, Project, TaskList, Task};
use dialoguer::Select;

pub struct InteractiveService<'a> {
    service: TeamWorkService<'a>,
    config: TeamWorkConfig<'a>,
}

impl<'a> InteractiveService<'a> {
    pub fn new<'b>(config: &'b TeamWorkConfig) -> InteractiveService<'b> {
        let service = TeamWorkService::new(&config);
        return InteractiveService {
            service: service.clone(),
            config: config.clone(),
        };
    }

    pub fn handle(&self) {
        let commands = &[
            InteractiveCommand::SeeStartTask,
            InteractiveCommand::SearchTask,
        ];

        let selected_action = Select::new()
            .with_prompt("What do you want to do ?")
            .items(commands)
            .default(0)
            .interact()
            .expect("Failed to get action");

        match &commands[selected_action] {
            InteractiveCommand::SeeStartTask => { println!("star") }
            InteractiveCommand::SearchTask => self.handle_search_task(),
        }
    }

    fn handle_search_task(&self) {
        let seach_opt: Option<String> = None;
        let projects = self.service.list_project(&seach_opt)
            .expect("Could not list projects")
            .projects;

        let selected_project = Select::new()
            .with_prompt("Choose a project ?")
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
            .items(tasks.as_slice())
            .default(0)
            .interact()
            .expect("Failed to get action");

        let task = tasks.get(select_task)
            .expect("Could not get selected selected task");

        self.handle_selected_task(&task.task);
    }

    fn handle_selected_task(&self, task: &Task) {
        let select_task = Select::new()
            .with_prompt("What do you want to do ?")
            .items(&[Commands::Back])
            .default(0)
            .interact()
            .expect("Failed to get action");
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

enum Commands {
    Back
}

impl ToString for Commands {
    fn to_string(&self) -> String {
        return match self {
            Commands::Back => "Go Back".to_string(),
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

/*let name = Input::<String>::new().with_prompt("Your name").interact()
.expect("Could not read name");
println!("Name: {}", name);

let select_project = Select::new()
.with_prompt("Projects ?")
.item("One")
.item("Two")
.items(&["Three", "Four"])
.interact()
.expect("Could not read project");
println!("Project: {}", select_project);

if Confirmation::new().with_text("Do you want to continue?").interact().expect("Oups") {
println!("Looks like you want to continue");
} else {
println!("nevermind then :(");
}*/

enum InteractiveCommand {
    SeeStartTask,
    SearchTask,
}

impl ToString for InteractiveCommand {
    fn to_string(&self) -> String {
        let str = match self {
            InteractiveCommand::SeeStartTask => "See start task",
            InteractiveCommand::SearchTask => "Search tasks",
        };

        return str.to_string();
    }
}
