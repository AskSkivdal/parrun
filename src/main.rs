use std::{fs, path::PathBuf, process::{Child, Command, Stdio}, thread, time::Duration};
use colored::Colorize;
use clap::Parser;

/// Run multiple commands at the same time.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The amount of processes to run at the same time
    #[arg(short, long, default_value_t = 6)]
    processes: usize,

    /// The amount of processes to run at the same time
    
    #[arg(short, long, default_value_t = String::from("sh"))]
    shell: String,

    /// Does not output the stdout of the commands
    #[arg(short, long, default_value_t = false)]
    quiet: bool,


    /// Arg file
    #[arg(short, long)]
    argfile: Option<PathBuf>,

    commands: Vec<String>
}

fn build_with_args(commands: Vec<String>, args: Vec<String>) -> Vec<String> {
    let mut command_list: Vec<String> = Vec::new();
    
    for arg in args {
        for command in &commands {
            command_list.push(command.replace("{{}}", &arg));
        }
    }

    command_list

}

fn build_command_queue(commands: Vec<String>, argfile: Option<PathBuf>) -> Result<Vec<String>, &'static str> {
    if let Some(argfile) = argfile {
        if !(argfile.exists() && argfile.is_file()) {
            return Err("No argfile")
        }

        let file: Vec<String> = fs::read_to_string(argfile).unwrap().split("\n").map(|x| x.to_string()).collect();

        return Ok(build_with_args(commands, file))

    } else {
        return Ok(commands)
    }
}

fn wait_for_free_child(children: &mut Vec<(Child, String)>) -> Vec<(Option<i32>, String)> {
    loop {

        let mut res: Vec<(Child, String)> = children.extract_if(.., |(x, _)| {
            match x.try_wait() {
                Ok(Some(_v)) => true,
                Ok(None) => false,
                Err(_) => true,
            }
        }).collect();

        let exit_codes: Vec<(Option<i32>, String)> = res.iter_mut().map(|(x, c)| ((*x).wait().unwrap().code(), c.clone())).collect();

        if !res.is_empty() {
            return exit_codes
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn main() {
    let args = Args::parse();
    let command_queue = build_command_queue(args.commands, args.argfile).unwrap();
    let limit = command_queue.len();

    let mut children: Vec<(Child, String)> = Vec::new();
    let mut none_zero_exist: Vec<(String, i32)> = Vec::new();
    println!("{}", format!("Starting {} processes", args.processes).green().bold());

    for (n, l) in command_queue.iter().enumerate() {
        if children.len() > args.processes {
            wait_for_free_child(&mut children);
        }

        println!("{}", format!("[{}/{limit}] Command started: {l}", n+1).green().bold());
        let cmd = Command::new(&args.shell)
            .args(["-c", &l])
            .stdout(if args.quiet {Stdio::null()} else {Stdio::inherit()})
            .spawn()
            .unwrap();

        children.push((cmd, l.clone()));
    }

    while children.len() != 0 {
        for (code, command) in wait_for_free_child(&mut children) {
            match code {
                // If the exit code is set to 0, then no action is needed
                Some(0) => continue,
                // If the exit code is set and is not 0 then add it to the non_zero exit codes to 
                // be logged at the end of the script.
                Some(v) => none_zero_exist.push((command.clone(), v)),
                // If the exit code isnt set, then do nothing.
                None => continue
            }
        }
    }

    for (command, status) in none_zero_exist {
        println!("{}", format!("Command: '{command}' exited with code '{status}'").red())
    }
}
