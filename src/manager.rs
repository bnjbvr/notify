use backend::prelude::{BackendError, PathBuf};
use std::fmt;
use super::{lifecycle::{LifeTrait, Status}, selector::{self, Selector}};
use tokio::{reactor::Handle, runtime::TaskExecutor};

pub struct Manager<'f> {
    pub handle: Handle,
    pub executor: TaskExecutor,
    pub selectors: Vec<Selector<'f>>,
    pub lives: Vec<Box<LifeTrait + 'f>>,
}

impl<'f> Manager<'f> {
    pub fn new(handle: Handle, executor: TaskExecutor) -> Self {
        Self {
            handle,
            executor,
            selectors: vec![],
            lives: vec![],
        }
    }

    pub fn add(&mut self, f: Selector<'f>) {
        self.selectors.push(f)
    }

    pub fn builtins(&mut self) {
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
        ))]
        self.add(Selector { f: &selector::inotify_life, name: "Inotify".into() });

        // #[cfg(any(
        //     target_os = "dragonfly",
        //     target_os = "freebsd",
        //     target_os = "netbsd",
        //     target_os = "openbsd",
        // ))]
        // self.add(Selector { f: &selector::kqueue_life, name: "Kqueue".into() });

        self.add(Selector { f: &selector::poll_life, name: "Poll".into() });
    }

    pub fn enliven(&mut self) {
        let mut lives = vec![];

        for sel in self.selectors.iter() {
            let mut l = (sel.f)(self.handle.clone(), self.executor.clone());
            l.with_name(sel.name.clone());

            if l.capabilities().len() > 0 {
                lives.push(l);
            }
        }

        self.lives = lives;
    }

    // TODO: figure out how to report and handle per-path errors

    pub fn bind(&mut self, paths: Vec<PathBuf>) -> Status {
        let mut err = None;
        for life in self.lives.iter_mut() {
            println!("Trying {:?}", life);
            match life.bind(paths.clone()) {
                Err(e) => {
                    println!("Got error: {:?}", e);
                    match e {
                        be @ BackendError::NonExistent(_) => return Err(be),
                        be @ _ => { err = Some(be); }
                    }
                },
                Ok(_) => return Ok(())
            }
        }

        match err {
            None => Err(BackendError::Unavailable(Some("No backend available".into()))),
            Some(e) => Err(e)
        }
    }

    pub fn active(&mut self) -> Option<&mut Box<LifeTrait + 'f>> {
        for life in self.lives.iter_mut() {
            if life.active() {
                return Some(life);
            }
        }

        None
    }
}

impl<'f> fmt::Debug for Manager<'f> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Manager")
            .field("handle", &self.handle)
            .field("selectors", &self.selectors)
            .field("lives", &self.lives)
            .finish()
    }
}