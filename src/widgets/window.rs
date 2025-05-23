use crate::utils::data_path;
use crate::widgets::{CollectionData, CollectionObject, TaskObject};
use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::property::PropertyGet;
use glib::Object;
use glib::{clone, subclass};
use gtk::{gio, glib, pango, CustomFilter, FilterListModel, NoSelection};

pub mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(gtk::CompositeTemplate, Default)]
    #[template(resource = "/com/github/shahradelahi/Todo/window.ui")]
    pub struct Window {
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub tasks_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub collections_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        pub collections: OnceCell<gio::ListStore>,
        pub current_collection: RefCell<Option<CollectionObject>>,
        pub current_filter_model: RefCell<Option<FilterListModel>>,
        pub tasks_changed_handler_id: RefCell<Option<glib::SignalHandlerId>>,

        pub settings: OnceCell<gio::Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "TodoWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("win.remove-done-tasks", None, |window, _, _| {
                window.remove_done_tasks();
            });

            klass.install_action_async("win.new-collection", None, |window, _, _| async move {
                window.new_collection().await;
            });
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.setup_settings();
            obj.setup_collections();
            obj.restore_data();
            obj.setup_callbacks();
            obj.setup_actions();
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {
        fn close_request(&self) -> glib::Propagation {
            let user_data = self
                .obj()
                .collections()
                .iter::<CollectionObject>()
                .filter_map(Result::ok)
                .map(|task_object| task_object.to_collection_data())
                .collect::<Vec<CollectionData>>();

            let file = std::fs::File::create(data_path()).expect("Could not create file");
            serde_json::to_writer(file, &user_data).expect("Could not write to file");

            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        Object::builder().property("application", app).build()
    }

    fn settings(&self) -> &gio::Settings {
        self.imp().settings.get().expect("Settings were not setup.")
    }

    fn tasks(&self) -> gio::ListStore {
        self.current_collection().tasks()
    }

    fn new_task(&self) {
        let buffer = self.imp().entry.buffer();
        let content = buffer.text().to_string();
        if content.is_empty() {
            return;
        }
        buffer.set_text("");

        let task = TaskObject::new(false, content);
        self.tasks().append(&task);
    }

    fn create_task_row(&self, task: &TaskObject) -> adw::ActionRow {
        let check_button = gtk::CheckButton::builder()
            .valign(gtk::Align::Center)
            .can_focus(false)
            .build();

        let row = adw::ActionRow::builder()
            .activatable_widget(&check_button)
            .build();
        row.add_prefix(&check_button);

        task.bind_property("completed", &check_button, "active")
            .bidirectional()
            .sync_create()
            .build();

        task.bind_property("content", &row, "title")
            .sync_create()
            .build();

        row
    }

    fn remove_done_tasks(&self) {
        self.tasks()
            .retain(|x| x.downcast_ref::<TaskObject>().unwrap().is_completed() != true);
    }

    async fn new_collection(&self) {
        let entry = gtk::Entry::builder()
            .placeholder_text("Name")
            .activates_default(true)
            .build();

        let cancel_response = "cancel";
        let create_response = "create";

        let dialog = adw::AlertDialog::builder()
            .heading("New Collection")
            .close_response(cancel_response)
            .default_response(create_response)
            .extra_child(&entry)
            .build();
        dialog.add_responses(&[(cancel_response, "Cancel"), (create_response, "Create")]);

        dialog.set_response_enabled(create_response, false);
        dialog.set_response_appearance(create_response, adw::ResponseAppearance::Suggested);

        entry.connect_changed(clone!(
            #[weak]
            dialog,
            move |entry| {
                let text = entry.text();
                let empty = text.is_empty();

                dialog.set_response_enabled(create_response, !empty);

                if empty {
                    entry.add_css_class("error");
                } else {
                    entry.remove_css_class("error");
                }
            }
        ));

        let response = dialog.choose_future(self).await;

        if response == cancel_response {
            return;
        }

        let tasks = gio::ListStore::new::<TaskObject>();

        let title = entry.text().to_string();
        let collection = CollectionObject::new(&title, tasks);

        self.collections().append(&collection);
        self.set_current_collection(collection);

        self.imp().split_view.set_show_content(true);
    }

    fn collections(&self) -> gio::ListStore {
        self.imp()
            .collections
            .get()
            .expect("Collections were not setup.")
            .clone()
    }

    fn current_collection(&self) -> CollectionObject {
        self.imp()
            .current_collection
            .borrow()
            .clone()
            .expect("No collection selected")
    }

    fn set_filter(&self) {
        self.imp()
            .current_filter_model
            .borrow()
            .clone()
            .expect("No filter model")
            .set_filter(self.filter().as_ref());
    }

    fn filter(&self) -> Option<CustomFilter> {
        let filter_done = CustomFilter::new(|object| {
            object
                .downcast_ref::<TaskObject>()
                .expect("Must be task object")
                .is_completed()
        });

        let filter_open = CustomFilter::new(|object| {
            !object
                .downcast_ref::<TaskObject>()
                .expect("Must be task object")
                .is_completed()
        });

        let filter_state = PropertyGet::get(&self.settings(), |settings| settings.string("filter"));

        match filter_state.as_str() {
            "All" => None,
            "Open" => Some(filter_done),
            "Done" => Some(filter_open),
            _ => unreachable!(),
        }
    }

    fn set_stack(&self) {
        if self.collections().n_items() > 0 {
            self.imp().stack.set_visible_child_name("main");
        } else {
            self.imp().stack.set_visible_child_name("placeholder");
        }
    }

    fn create_collection_row(&self, collection_object: &CollectionObject) -> gtk::ListBoxRow {
        let label = gtk::Label::builder()
            .ellipsize(pango::EllipsizeMode::End)
            .xalign(0.0)
            .build();

        collection_object
            .bind_property("title", &label, "label")
            .sync_create()
            .build();

        gtk::ListBoxRow::builder().child(&label).build()
    }

    fn select_collection_row(&self) {
        if let Some(index) = self.collections().find(&self.current_collection()) {
            let row = self.imp().collections_list.row_at_index(index as i32);
            self.imp().collections_list.select_row(row.as_ref());
        }
    }

    fn set_task_list_visibility(&self, tasks: &gio::ListStore) {
        // Assure that the task list is only visible when there is at least one task
        self.imp().tasks_list.set_visible(tasks.n_items() > 0);
    }

    fn restore_data(&self) {
        if let Ok(file) = std::fs::File::open(data_path()) {
            let data: Vec<CollectionData> =
                serde_json::from_reader(file).expect("Could not read file");

            let collections = data
                .into_iter()
                .map(CollectionObject::from_collection_data)
                .collect::<Vec<CollectionObject>>();

            self.collections().extend_from_slice(&collections);

            if let Some(collection) = collections.first() {
                self.set_current_collection(collection.clone());
            }
        }
    }

    fn set_current_collection(&self, collection: CollectionObject) {
        let tasks = collection.tasks();
        let filter_model = FilterListModel::new(Some(tasks.clone()), self.filter());
        let selection_model = NoSelection::new(Some(filter_model.clone()));

        self.imp().tasks_list.bind_model(
            Some(&selection_model),
            clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or_panic]
                move |obj| {
                    let task_object = obj
                        .downcast_ref::<TaskObject>()
                        .expect("Must be task object");

                    let row = window.create_task_row(task_object);
                    row.upcast()
                }
            ),
        );

        // Store filter model
        self.imp().current_filter_model.replace(Some(filter_model));

        // If present, disconnect old `tasks_changed` handler
        if let Some(handler_id) = self.imp().tasks_changed_handler_id.take() {
            self.tasks().disconnect(handler_id);
        }

        // Set current tasks
        self.imp().current_collection.replace(Some(collection));

        self.select_collection_row();

        self.set_task_list_visibility(&tasks);
        let tasks_changed_handler_id = self.tasks().connect_items_changed(clone!(
            #[weak(rename_to = window)]
            self,
            move |tasks, _, _, _| {
                window.set_task_list_visibility(&tasks);
            }
        ));
        self.imp()
            .tasks_changed_handler_id
            .replace(Some(tasks_changed_handler_id));
    }

    fn setup_collections(&self) {
        let collections = gio::ListStore::new::<CollectionObject>();
        self.imp()
            .collections
            .set(collections.clone())
            .expect("Collections already set.");

        self.imp().collections_list.bind_model(
            Some(&collections),
            clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or_panic]
                move |obj| {
                    let collection_object = obj
                        .downcast_ref::<CollectionObject>()
                        .expect("Must be collection object");

                    let row = window.create_collection_row(collection_object);
                    row.upcast()
                }
            ),
        );
    }

    fn setup_callbacks(&self) {
        self.settings().connect_changed(
            Some("filter"),
            clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.set_filter();
                }
            ),
        );

        self.set_stack();
        self.collections().connect_items_changed(clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.set_stack();
            }
        ));

        self.imp().collections_list.connect_row_activated(clone!(
            #[weak(rename_to = window)]
            self,
            move |_, row| {
                let index = row.index();
                let selection_collection = window
                    .collections()
                    .item(index as u32)
                    .expect("Collection not found")
                    .downcast::<CollectionObject>()
                    .expect("Must be collection object");
                window.set_current_collection(selection_collection);
                window.imp().split_view.set_show_content(true);
            }
        ));

        self.imp().entry.connect_activate(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.new_task();
            }
        ));

        self.imp().entry.connect_icon_release(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _| {
                window.new_task();
            }
        ));
    }

    fn setup_settings(&self) {
        let settings = gio::Settings::new(crate::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("Settings already set.");
    }

    fn setup_actions(&self) {
        let action_filter = self.settings().create_action("filter");
        self.add_action(&action_filter);

        let action_close = gio::ActionEntry::builder("close")
            .activate(|window: &Window, _, _| {
                window.close();
            })
            .build();
        self.add_action_entries([action_close]);

        let action_orientation = gio::ActionEntry::builder("orientation")
            .parameter_type(Some(&String::static_variant_type()))
            .state("Vertical".into())
            .activate(|_, _, parameter| {
                let parameter = parameter
                    .expect("Orientation parameter not found")
                    .get::<String>()
                    .expect("Orientation parameter is not a string");

                let orientation = match parameter.as_str() {
                    "Vertical" => gtk::Orientation::Vertical,
                    "Horizontal" => gtk::Orientation::Horizontal,
                    _ => unreachable!(),
                };

                println!("Orientation: {:?}", orientation);
            })
            .build();
        self.add_action_entries([action_orientation]);
    }
}
