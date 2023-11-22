mod logger;
#[macro_use]
mod scaffolding;
mod server;

use std::env;
use std::error::Error;

use scaffolding::Context;

problem_list! {
    smoke_test
    prime_time
    means_to_an_end
    budget_chat
}

fn main() -> Result<(), Box<dyn Error>> {
    logger::init()?;
    let ctx = Context::new(
        env::args().collect(),
        env::var("BIND_ADDRESS").unwrap_or(String::from("127.0.0.1:0")),
    );

    let handler = match ctx.problem.as_deref() {
        None => handle_no_problem_specified,
        Some("help") => match ctx.problem_arguments.get(0) {
            None => handle_basic_help,
            Some(problem) => get_problem_help(problem).unwrap_or(handle_help_for_unknown_problem),
        },
        Some(problem) => get_problem_handler(problem).unwrap_or(handle_problem_not_found),
    };

    handler(&ctx)
}

fn print_available_problems(ctx: &Context) {
    println!("Usage: {} <problem_name> [...]", ctx.program_name);
    println!("Available problems:");
    for problem_name in get_problem_names() {
        println!("  {}", problem_name);
    }
}

fn handle_no_problem_specified(ctx: &Context) -> Result<(), Box<dyn Error>> {
    print_available_problems(ctx);
    Err(String::from("No problem specified.").into())
}

fn handle_basic_help(ctx: &Context) -> Result<(), Box<dyn Error>> {
    print_available_problems(ctx);
    Ok(())
}

fn handle_help_for_unknown_problem(ctx: &Context) -> Result<(), Box<dyn Error>> {
    print_available_problems(ctx);
    Err(format!(
        "Problem '{}' not found.",
        ctx.problem_arguments
            .get(0)
            .expect("We are here precisely because this is set")
    )
    .into())
}

fn handle_problem_not_found(ctx: &Context) -> Result<(), Box<dyn Error>> {
    print_available_problems(ctx);
    Err(format!(
        "Problem '{}' not found.",
        ctx.problem
            .as_ref()
            .expect("We are here precisely because this is set")
    )
    .into())
}
