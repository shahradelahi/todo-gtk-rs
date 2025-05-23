use adw::prelude::*;
use serde::{Deserialize, Serialize};

use crate::widgets::{TaskData, TaskObject};

pub mod imp {
    use adw::subclass::prelude::*;
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(glib::Properties, Default)]
    #[properties(wrapper_type = super::CollectionObject)]
    pub struct CollectionObject {
        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub tasks: OnceCell<gio::ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CollectionObject {
        const NAME: &'static str = "TodoCollectionObject";
        type Type = super::CollectionObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for CollectionObject {}
}

glib::wrapper! {
    pub struct CollectionObject(ObjectSubclass<imp::CollectionObject>);
}

impl CollectionObject {
    pub fn new(title: &str, tasks: gio::ListStore) -> Self {
        glib::Object::builder()
            .property("title", title)
            .property("tasks", tasks)
            .build()
    }

    pub fn to_collection_data(&self) -> CollectionData {
        let title = self.title();
        let tasks_data = self
            .tasks()
            .iter::<TaskObject>()
            .filter_map(Result::ok)
            .map(|task| task.to_task_data())
            .collect();
        CollectionData { title, tasks_data }
    }

    pub fn from_collection_data(data: CollectionData) -> Self {
        let title = data.title;
        let tasks = data
            .tasks_data
            .into_iter()
            .map(TaskObject::from_task_data)
            .collect::<Vec<TaskObject>>();

        let tasks_store = gio::ListStore::new::<TaskObject>();
        tasks_store.extend_from_slice(&tasks);

        Self::new(&title, tasks_store)
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct CollectionData {
    pub title: String,
    pub tasks_data: Vec<TaskData>,
}
