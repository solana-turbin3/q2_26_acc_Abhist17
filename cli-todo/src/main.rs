use clap::*;
use borsh::{BorshSerialize, BorshDeserialize};
use std::collections::VecDeque;
use std::io::Write;
use std::time::SystemTime;
use std::fs::File;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
struct Todo {
    id: u64,
    title: String,
    created_at: u64,      
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Queue<T> {
    items: VecDeque<T>,
}

impl<T> Queue<T>
where T : BorshSerialize + BorshDeserialize + std::fmt::Debug
{
    fn new() -> Self {
        Queue {
            items: VecDeque::new(),
        }
    }

    fn load() -> Self {
        let mut file = match File::open("todo.bin") {
            Ok(file) => file,
            Err(_) => {
                println!("Creating new file");
                match File::create("todo.bin") {
                    Ok(file) => file,
                    Err(_) => {
                        println!("Failed to create file");
                        std::process::exit(1);
                    }
                }
            }
        };

        match Queue::try_from_reader(&mut file) {
            Ok(queue) => queue,
            Err(_) => {
                println!("Error deserializing file");
                std::process::exit(1);
            }
        }
    }

    fn save(&self) {
        let mut writer = Vec::new();
        match self.serialize(&mut writer) {
            Ok(_) => {},
            Err(_) => {
                println!("Error serializing queue");
                std::process::exit(1);
            }
        }

        let mut file = match File::create("todo.bin") {
            Ok(file) => file,
            Err(_) => {
                println!("Failed to open file for writing");
                std::process::exit(1);
            }
        };

        match file.write_all(&writer) {
            Ok(_) => println!("Written successfully"),
            Err(_) => {
                println!("Error writing to file");
                std::process::exit(1);
            }
        }
    }

    fn len(&self) -> u64 {
        self.items.len() as u64
    }

    fn enqueue(&mut self, item: T) {
        self.items.push_back(item);
    }

    fn dequeue(&mut self) -> Option<T> {
        println!("Dequeuing item");
        match self.items.pop_front() {
            Some(item) => Some(item),
            None => {
                println!("No task to complete");
                None
            }
        }
    }

    fn peek(&self) -> Option<&T> {
        self.items.front()
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn clear(&mut self) {
        self.items.clear();
    }

    fn print(&self) {
        if self.items.len() == 0 {
            println!("No tasks to list");
        } else {
            for item in &self.items {
                println!("{:?}", item);
            }
        }
    }
}

impl Queue<Todo> {
    fn next_id(&self) -> u64 {
        self.peek()
            .map(|t| t.id + self.len())
            .unwrap_or(1)
    }
}

#[derive(Parser, Debug)]
#[command(name = "cli-todo", version = "0.1.0", author = "Vedansh")]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a task to the list
    Add {
        /// The task to add
        title: String
    },
    /// List all the tasks
    List,
    /// Complete the oldest task
    Complete,
}

fn main() {

    let args = Args::parse();

    let mut queue = Queue::load();

    if let Commands::Add { title } = args.command {
        println!("Adding todo: {}", title);
        
        let todo = Todo {
            id: queue.next_id(),
            title: title.clone(),
            created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
        };

        queue.enqueue(todo);

        queue.save();
        
    } else if let Commands::List = args.command {
        queue.print()
    } else if let Commands::Complete = args.command {
        queue.dequeue();
        queue.save();
    }
}