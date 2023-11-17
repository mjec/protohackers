use std::collections::VecDeque;

pub(crate) struct Context {
    pub(crate) program_name: String,
    pub(crate) problem: Option<String>,
    pub(crate) problem_arguments: VecDeque<String>,
    pub(crate) bind_address: String,
}

impl Context {
    pub(crate) fn new(mut args: VecDeque<String>, bind_address: String) -> Self {
        let program_name = args.pop_front().expect("We expect argv[0]");
        let problem = args.pop_front();
        let problem_arguments = args;
        Self {
            program_name,
            problem,
            problem_arguments,
            bind_address,
        }
    }
}

/// Generate boilerplate for each problem, permitting dispatch between them.
/// Requires a whitespace-separated list of problems. Each problem must have
/// a module of the same name, which should NOT have a `mod` statement otherwise.
///
/// Each module must have two functions with the following signatures:
///
///     pub(crate) fn run(ctx: &Context) -> Result<(), Box<dyn std::error::Error>>
///     pub(crate) fn help(ctx: &Context) -> Result<(), Box<dyn std::error::Error>>
macro_rules! problem_list {
    { $($name:ident)+ } => {
        $(mod $name;)+

        fn get_problem_handler(problem_name: &str) -> Option<fn(&Context) -> Result<(), Box<dyn Error>>> {
            match problem_name {
                $(stringify!($name) => Some($name::run),)+
                _ => None,
            }
        }

        fn get_problem_help(problem_name: &str) -> Option<fn(&Context) -> Result<(), Box<dyn Error>>> {
            match problem_name {
                $(stringify!($name) => Some($name::help),)+
                _ => None,
            }
        }

        fn get_problem_names() -> Vec<&'static str> {
            vec![$(stringify!($name)),+]
        }
    };
}
