use failure::Fail;

/// Errors related to schedulers/scheduling
// TODO: extend this, as we probably want more error handling over
//       scheduling
#[derive(Debug, Fail)]
pub enum SchedulerError {
    #[fail(display = "No scheduler running on core {}", _0)]
    NoRunningSchedulerOnCore(i32),
}

pub trait Executable {
    fn execute(&mut self);
    fn dependencies(&mut self) -> Vec<usize>;
}

impl<F> Executable for F
where
    F: FnMut(),
{
    fn execute(&mut self) {
        (*self)()
    }

    fn dependencies(&mut self) -> Vec<usize> {
        vec![]
    }
}
