use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use serde::{Deserialize, Serialize};

pub mod imp {
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    use super::*;

    #[derive(glib::Properties, Default)]
    #[properties(wrapper_type = super::TaskObject)]
    pub struct TaskObject {
        #[property(name = "completed", get, set, type = bool, member = completed)]
        #[property(name = "content", get, set, type = String, member = content)]
        pub data: RefCell<TaskData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TaskObject {
        const NAME: &'static str = "TaskObject";
        type Type = super::TaskObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for TaskObject {}
}

glib::wrapper! {
    pub struct TaskObject(ObjectSubclass<imp::TaskObject>);
}

impl TaskObject {
    pub fn new(completed: bool, content: String) -> Self {
        Object::builder()
            .property("completed", completed)
            .property("content", content)
            .build()
    }

    pub fn from_task_data(data: TaskData) -> Self {
        Self::new(data.completed, data.content)
    }

    pub fn to_task_data(&self) -> TaskData {
        self.imp().data.borrow().clone()
    }

    pub fn is_completed(&self) -> bool {
        self.imp().data.borrow().completed
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct TaskData {
    pub completed: bool,
    pub content: String,
}
